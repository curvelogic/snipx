use clap::{Parser, Subcommand, ValueEnum};
use snipx_core::{format, FormatOptions, InputForm};
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

#[derive(Debug, Parser)]
#[command(name = "snipx")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Check,
    Resolve,
    Export,
    Fmt(FmtArgs),
}

#[derive(Debug, Parser)]
struct FmtArgs {
    #[arg(long = "as", value_name = "FORM")]
    input_form: Option<CliInputForm>,

    #[arg(short = 'c')]
    commentaria: bool,

    #[arg(short = 'm')]
    marginalia: bool,

    #[arg(short = 'i')]
    intralinea: bool,

    #[arg(long)]
    write: bool,

    path: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum CliInputForm {
    Commentaria,
    Marginalia,
    Intralinea,
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

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            let _ = writeln!(io::stderr(), "{}", error.message);
            ExitCode::from(error.code)
        }
    }
}

fn run() -> Result<(), CliError> {
    let cli = Cli::parse();

    match cli.command {
        Command::Check | Command::Resolve | Command::Export => Ok(()),
        Command::Fmt(args) => run_fmt(args),
    }
}

fn run_fmt(args: FmtArgs) -> Result<(), CliError> {
    let input_form = select_input_form(&args)?;
    let input = read_input(args.path.as_ref())?;
    let result = format(&input, FormatOptions { input_form });

    if args.write {
        let path = args
            .path
            .as_ref()
            .ok_or_else(|| CliError::usage("--write requires a path argument"))?;
        fs::write(path, result.output).map_err(|source| CliError::io(path, source))?;
    } else {
        print!("{}", result.output);
    }

    Ok(())
}

fn select_input_form(args: &FmtArgs) -> Result<InputForm, CliError> {
    let mut selected = Vec::new();

    if let Some(input_form) = args.input_form {
        selected.push(InputForm::from(input_form));
    }
    if args.commentaria {
        selected.push(InputForm::Commentaria);
    }
    if args.marginalia {
        selected.push(InputForm::Marginalia);
    }
    if args.intralinea {
        selected.push(InputForm::Intralinea);
    }

    match selected.len() {
        0 => Ok(InputForm::Commentaria),
        1 => Ok(selected[0]),
        _ if selected.iter().all(|input_form| *input_form == selected[0]) => Ok(selected[0]),
        _ => Err(CliError::usage("conflicting input form selectors")),
    }
}

fn read_input(path: Option<&PathBuf>) -> Result<String, CliError> {
    match path {
        Some(path) => fs::read_to_string(path).map_err(|source| CliError::io(path, source)),
        None => {
            let mut input = String::new();
            io::stdin()
                .read_to_string(&mut input)
                .map_err(|source| CliError {
                    code: 1,
                    message: format!("failed to read stdin: {source}"),
                })?;
            Ok(input)
        }
    }
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
            code: 1,
            message: format!("{}: {source}", path.display()),
        }
    }
}
