use clap::{ArgAction, Args, Parser, Subcommand, ValueEnum};
use snipx_core::ast::AstNode;
use snipx_core::{
    expand, export_json, format, parse, ExpandOptions, ExportRequest, FormatOptions, InputForm,
    ParseOptions, Profile, Value,
};
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

#[derive(Debug, Parser)]
#[command(name = "snipx", version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Check(DocumentArgs),
    Resolve(DocumentArgs),
    Export(DocumentArgs),
    Fmt(FmtArgs),
}

#[derive(Debug, Args)]
struct DocumentArgs {
    #[command(flatten)]
    input: InputFormArgs,

    #[arg(long, value_name = "PATH")]
    target: Option<PathBuf>,

    #[arg(long, value_enum)]
    profile: Option<CliProfile>,

    #[arg(long, value_name = "EXPR", allow_hyphen_values = true)]
    ambient: Option<String>,

    #[arg(long)]
    pretty: bool,

    #[arg(long)]
    strict: bool,

    path: Option<PathBuf>,
}

#[derive(Debug, Args)]
struct FmtArgs {
    #[command(flatten)]
    input: InputFormArgs,

    #[arg(long)]
    write: bool,

    path: Option<PathBuf>,
}

#[derive(Debug, Args)]
struct InputFormArgs {
    #[arg(long = "as", value_name = "FORM")]
    input_form: Vec<CliInputForm>,

    #[arg(short = 'c', action = ArgAction::Count)]
    commentaria: u8,

    #[arg(short = 'm', action = ArgAction::Count)]
    marginalia: u8,

    #[arg(short = 'i', action = ArgAction::Count)]
    intralinea: u8,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum CliInputForm {
    Commentaria,
    Marginalia,
    Intralinea,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum CliProfile {
    Plain,
    PlainLoose,
    Markdown,
    MarkdownLoose,
}

impl From<CliInputForm> for InputForm {
    fn from(value: CliInputForm) -> Self {
        match value {
            CliInputForm::Commentaria => Self::Commentaria,
            CliInputForm::Marginalia => Self::Marginalia,
            CliInputForm::Intralinea => Self::Intralinea,
        }
    }
}

impl From<CliProfile> for Profile {
    fn from(value: CliProfile) -> Self {
        match value {
            CliProfile::Plain => Self::Plain,
            CliProfile::PlainLoose => Self::PlainLoose,
            CliProfile::Markdown => Self::Markdown,
            CliProfile::MarkdownLoose => Self::MarkdownLoose,
        }
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            let _ = writeln!(io::stderr(), "{}", error.message);
            ExitCode::from(error.code)
        }
    }
}

fn run() -> Result<u8, CliError> {
    let cli = Cli::parse();

    match cli.command {
        Command::Check(args) => run_document(args, OutputView::Check),
        Command::Resolve(args) => run_document(args, OutputView::Resolve),
        Command::Export(args) => run_document(args, OutputView::Export),
        Command::Fmt(args) => run_fmt(args),
    }
}

/// Which slice of the canonical JSON document a command reports.
/// `check` validates, `resolve` adds resolution results, `export`
/// emits the full document including facts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutputView {
    Check,
    Resolve,
    Export,
}

fn run_document(args: DocumentArgs, view: OutputView) -> Result<u8, CliError> {
    let input_form = select_input_form(&args.input)?;
    let profile = args.profile.map(Profile::from);

    let source_uses_stdin = args
        .path
        .as_deref()
        .is_none_or(|path| path == Path::new("-"));
    let target_uses_stdin = args.target.as_deref() == Some(Path::new("-"));
    if source_uses_stdin && target_uses_stdin {
        return Err(CliError::usage(
            "source and target cannot both read from stdin",
        ));
    }

    let source = read_input(args.path.as_deref())?;
    let target_text = match args.target.as_deref() {
        Some(path) => Some(read_input(Some(path))?),
        None if input_form == InputForm::Commentaria => header_target_directive(&source)
            .map(|value| {
                let path = resolve_directive_target(&value, args.path.as_deref());
                read_input(Some(&path))
            })
            .transpose()?,
        None => None,
    };
    let target_uri = args
        .target
        .as_ref()
        .map(|path| path.to_string_lossy().into_owned());
    let ambient_subject = args.ambient.as_deref().map(parse_ambient).transpose()?;

    let document = export_json(ExportRequest {
        source,
        input_form,
        target_text,
        profile,
        path: args
            .path
            .as_ref()
            .filter(|path| path.as_path() != Path::new("-"))
            .map(|path| path.to_string_lossy().into_owned()),
        target_uri,
        ambient_subject,
    });

    let mut value = serde_json::to_value(&document).map_err(|source| CliError {
        code: 1,
        message: format!("failed to serialise JSON: {source}"),
    })?;
    if let Some(object) = value.as_object_mut() {
        match view {
            OutputView::Check => {
                object.remove("facts");
                object.remove("resolutions");
                object.remove("visibleText");
            }
            OutputView::Resolve => {
                object.remove("facts");
            }
            OutputView::Export => {}
        }
    }
    let output = if args.pretty {
        serde_json::to_string_pretty(&value)
    } else {
        serde_json::to_string(&value)
    }
    .map_err(|source| CliError {
        code: 1,
        message: format!("failed to serialise JSON: {source}"),
    })?;
    writeln!(io::stdout(), "{output}").map_err(CliError::stdout)?;

    if document.has_unsupported_features() {
        Ok(4)
    } else if document.has_errors() || (args.strict && document.has_warnings()) {
        Ok(1)
    } else {
        Ok(0)
    }
}

fn run_fmt(args: FmtArgs) -> Result<u8, CliError> {
    let input_form = select_input_form(&args.input)?;
    let write_path = if args.write {
        Some(
            args.path
                .as_ref()
                .filter(|path| path.as_path() != Path::new("-"))
                .ok_or_else(|| CliError::usage("--write requires a path argument"))?,
        )
    } else {
        None
    };
    let input = read_input(args.path.as_deref())?;
    let result = format(&input, FormatOptions { input_form });
    let has_errors = result
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == snipx_core::Severity::Error);

    if let Some(path) = write_path {
        fs::write(path, result.output).map_err(|source| CliError::io(path, source))?;
    } else {
        write!(io::stdout(), "{}", result.output).map_err(CliError::stdout)?;
    }

    Ok(u8::from(has_errors))
}

fn select_input_form(args: &InputFormArgs) -> Result<InputForm, CliError> {
    let mut selected = Vec::new();

    for input_form in &args.input_form {
        selected.push(InputForm::from(*input_form));
    }
    for _ in 0..args.commentaria {
        selected.push(InputForm::Commentaria);
    }
    for _ in 0..args.marginalia {
        selected.push(InputForm::Marginalia);
    }
    for _ in 0..args.intralinea {
        selected.push(InputForm::Intralinea);
    }

    match selected.len() {
        0 => Ok(InputForm::Commentaria),
        1 => Ok(selected[0]),
        _ if selected.iter().all(|input_form| *input_form == selected[0]) => Ok(selected[0]),
        _ => Err(CliError::usage("conflicting input form selectors")),
    }
}

fn read_input(path: Option<&Path>) -> Result<String, CliError> {
    match path {
        Some(path) if path != Path::new("-") => {
            fs::read_to_string(path).map_err(|source| CliError::io(path, source))
        }
        _ => {
            let mut input = String::new();
            io::stdin()
                .read_to_string(&mut input)
                .map_err(|source| CliError {
                    code: 3,
                    message: format!("failed to read stdin: {source}"),
                })?;
            Ok(input)
        }
    }
}

fn header_target_directive(source: &str) -> Option<String> {
    let parsed = parse(
        source,
        ParseOptions {
            input_form: InputForm::Commentaria,
        },
    );
    let root = snipx_core::ast::Root::cast(parsed.syntax().clone())?;
    root.header_directives()
        .target
        .map(|directive| directive.value)
}

fn resolve_directive_target(value: &str, source_path: Option<&Path>) -> PathBuf {
    let value = value.strip_prefix("file://").unwrap_or(value);
    let target = Path::new(value);
    if target.is_absolute() {
        return target.to_path_buf();
    }
    match source_path {
        Some(path) if path != Path::new("-") => path
            .parent()
            .map(|parent| parent.join(target))
            .unwrap_or_else(|| target.to_path_buf()),
        _ => target.to_path_buf(),
    }
}

fn parse_ambient(expression: &str) -> Result<Value, CliError> {
    if expression == "[]" {
        return Ok(Value::WholeDocument);
    }
    if let Ok(value) = expression.parse::<f64>() {
        if !value.is_finite() {
            return Err(CliError::usage("ambient number must be finite"));
        }
        return Ok(Value::Number(value));
    }
    if expression.is_empty() {
        return Err(CliError::usage("ambient subject cannot be empty"));
    }

    let source = format!("{expression} ambient Ambient.\n");
    let parsed = parse(
        &source,
        ParseOptions {
            input_form: InputForm::Commentaria,
        },
    );
    if !parsed.diagnostics().is_empty() {
        return Err(CliError::usage("invalid ambient expression"));
    }
    let mut expanded = expand(&parsed, ExpandOptions::default());
    if !expanded.diagnostics.is_empty() || expanded.statements.len() != 1 {
        return Err(CliError::usage("invalid ambient expression"));
    }
    let statement = expanded
        .statements
        .pop()
        .expect("one expanded statement was checked above");
    let consumes_expression = statement
        .subject_span
        .as_ref()
        .is_some_and(|span| span.start == 0 && span.end == expression.len());
    if !consumes_expression {
        return Err(CliError::usage("invalid ambient expression"));
    }
    Ok(statement.subject)
}

#[derive(Debug)]
struct CliError {
    code: u8,
    message: String,
}

impl CliError {
    fn usage(message: impl Into<String>) -> Self {
        Self {
            code: 2,
            message: message.into(),
        }
    }

    fn io(path: &Path, source: io::Error) -> Self {
        Self {
            code: 3,
            message: format!("{}: {source}", path.display()),
        }
    }

    fn stdout(source: io::Error) -> Self {
        Self {
            code: 3,
            message: format!("failed to write stdout: {source}"),
        }
    }

    #[allow(dead_code)] // Reserved for stable unsupported-feature diagnostics (exit code 4).
    fn unsupported(message: impl Into<String>) -> Self {
        Self {
            code: 4,
            message: message.into(),
        }
    }
}
