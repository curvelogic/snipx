# Reference Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the crate-first Rust reference implementation for SnipX, with a reusable `snipx-core` crate, a thin `snipx` CLI, conservative formatting, JSON output, plain text resolution, Markdown extraction, CI, parser property tests, fuzzing, and Beads issue tracking.

**Architecture:** Use a Rust workspace with `crates/snipx-core` as the main implementation surface and `crates/snipx` as the CLI driver. Implement the whole grammar for commentaria, marginalia, and intralinea before hardening resolution and export semantics. Use a lossless Rowan CST with typed AST/query wrappers, then layer formatting, expansion, resolution, and JSON export over it.

**Tech Stack:** Rust 2021, `rowan`, `serde`, `serde_json`, `clap`, `unicode-normalization`, `insta`, `assert_cmd`, `predicates`, `proptest`, `cargo-fuzz`, and later `pulldown-cmark`.

## Global Constraints

- The implementation is crate-first: `snipx-core` is the primary public surface, and `snipx` is a thin CLI.
- The parser must cover commentaria, marginalia, and intralinea from the first parser milestone.
- Parser internals are implementation taste, but the output must be a lossless Rowan CST with recoverable diagnostics.
- The formatter is in v0 and must preserve marginalia prose and intralinea host text byte-for-byte outside SnipX syntax regions.
- Plain text extraction and resolution come before Markdown extraction.
- Export is partial and diagnostic-rich: unresolved snippets remain represented instead of silently suppressing facts.
- Dependencies are advisory until implementation choices prove them.
- Comprehensive GitHub Actions CI must run formatting, clippy, and tests.
- Beads is the durable task tracker: create parent epics and child tasks corresponding to this plan before implementation begins.
- The design source is `docs/superpowers/specs/2026-07-07-reference-implementation-design.md`.
- The language source is `docs/language-spec.md`.

---

## File Structure

- Create `.github/workflows/ci.yml`: CI for Rust format, clippy, test, and later fuzz smoke checks.
- Create `Cargo.toml`: workspace root.
- Create `crates/snipx-core/Cargo.toml`: core crate dependencies and features.
- Create `crates/snipx-core/src/lib.rs`: public module exports and top-level API types.
- Create `crates/snipx-core/src/input.rs`: `InputForm`, `ParseOptions`, `FormatOptions`, `ResolveOptions`, and shared configuration.
- Create `crates/snipx-core/src/diagnostic.rs`: stable diagnostic codes, severities, spans, and reporting types.
- Create `crates/snipx-core/src/syntax.rs`: Rowan language, syntax kinds, tokens, nodes, and aliases.
- Create `crates/snipx-core/src/parser.rs`: lossless parser entry points.
- Create `crates/snipx-core/src/ast.rs`: typed AST/query wrappers over CST nodes.
- Create `crates/snipx-core/src/format.rs`: conservative SnipX-region formatter.
- Create `crates/snipx-core/src/expand.rs`: statement and sugar expansion.
- Create `crates/snipx-core/src/visible_text.rs`: canonical visible-text extraction.
- Create `crates/snipx-core/src/match.rs`: exact and loose snippet matching.
- Create `crates/snipx-core/src/resolve.rs`: snippet resolution and unresolved value representation.
- Create `crates/snipx-core/src/json.rs`: canonical JSON serialisable structures.
- Create `crates/snipx-core/tests/parser_snapshots.rs`: parser fixture tests.
- Create `crates/snipx-core/tests/formatter_snapshots.rs`: formatter fixture tests.
- Create `crates/snipx-core/tests/expansion.rs`: expansion unit tests.
- Create `crates/snipx-core/tests/resolution.rs`: visible text and snippet resolution tests.
- Create `crates/snipx-core/tests/json_snapshots.rs`: canonical JSON snapshot tests.
- Create `crates/snipx-core/tests/parser_properties.rs`: property tests for parser and formatter invariants.
- Create `crates/snipx/Cargo.toml`: CLI crate dependencies.
- Create `crates/snipx/src/main.rs`: CLI entry point.
- Create `crates/snipx/tests/cli.rs`: CLI integration tests.
- Create `fuzz/Cargo.toml` and `fuzz/fuzz_targets/parser.rs`: fuzz harness after the parser exists.

---

### Task 1: Create Beads Issue Hierarchy

**Files:**
- No repository files are modified by this task.
- Beads database: `.beads/embeddeddolt`

**Interfaces:**
- Consumes: `docs/superpowers/plans/2026-07-07-reference-implementation.md`
- Produces: Parent epics and child tasks in Beads, with dependencies matching this plan.

- [ ] **Step 1: Create parent epics**

Run:

```bash
bd create --title="Reference implementation: workspace, parser, and formatter" --description="Parent epic for Rust workspace setup, complete lossless parser, typed AST/query layer, and conservative formatter." --type=epic --priority=1
bd create --title="Reference implementation: expansion, resolution, and JSON" --description="Parent epic for statement expansion, diagnostics, visible-text extraction, snippet matching, resolution, canonical JSON, and CLI hardening." --type=epic --priority=1
bd create --title="Reference implementation: Markdown, CI, property tests, and fuzzing" --description="Parent epic for Markdown visible-text extraction, comprehensive CI, parser property tests, fuzzing, and implementation hardening." --type=epic --priority=2
```

Expected: three new Beads IDs are printed.

- [ ] **Step 2: Create child task Beads**

Run one `bd create` per implementation task, using the corresponding parent epic IDs from Step 1:

```bash
bd create --title="Scaffold Rust workspace and CI" --description="Create workspace, core crate, CLI crate, and GitHub Actions checks for fmt, clippy, and tests." --type=task --priority=1
bd create --title="Implement lossless syntax infrastructure" --description="Create syntax kinds, Rowan language wrapper, diagnostics, and parser entry result types." --type=task --priority=1
bd create --title="Implement full parser for all input forms" --description="Parse commentaria, marginalia, and intralinea including snippets, statements, fences, comments, strings, decorations, and local subject markers." --type=task --priority=1
bd create --title="Implement typed AST query layer and formatter" --description="Expose typed CST wrappers and conservative formatting for SnipX regions only." --type=task --priority=1
bd create --title="Implement expansion and diagnostics" --description="Expand Turtle-style carry-forward, ambient subjects, and decoration sugar with stable diagnostics." --type=task --priority=1
bd create --title="Implement plain visible text and snippet resolution" --description="Implement plain/plain-loose profiles, matching, captures, ranges, quantifiers, and unresolved snippet values." --type=task --priority=1
bd create --title="Implement canonical JSON and CLI commands" --description="Implement check, resolve, export, and fmt commands using canonical JSON and documented exit codes." --type=task --priority=1
bd create --title="Implement Markdown visible text extraction" --description="Add markdown/markdown-loose profiles after the plain path is working." --type=task --priority=2
bd create --title="Add parser property tests and fuzzing" --description="Add proptest invariants and cargo-fuzz parser/formatter harnesses, promoting failures to fixtures." --type=task --priority=2
```

Expected: each command prints a new Beads ID.

- [ ] **Step 3: Link child tasks to epics**

Run `bd dep add <child> <parent>` for each child task and its parent epic. Use the Beads IDs created in Steps 1 and 2.

Expected: `bd show <child>` lists the parent epic as a dependency or linked blocker according to Beads output.

- [ ] **Step 4: Sync Beads**

Run:

```bash
bd dolt push
```

Expected: Dolt push succeeds and remote Beads state contains the new hierarchy.

---

### Task 2: Scaffold Workspace, Crates, and CI

**Files:**
- Create: `Cargo.toml`
- Create: `crates/snipx-core/Cargo.toml`
- Create: `crates/snipx-core/src/lib.rs`
- Create: `crates/snipx-core/src/input.rs`
- Create: `crates/snipx-core/src/diagnostic.rs`
- Create: `crates/snipx/Cargo.toml`
- Create: `crates/snipx/src/main.rs`
- Create: `.github/workflows/ci.yml`

**Interfaces:**
- Produces: `snipx_core::{InputForm, ParseOptions, ParseResult, Diagnostic}` and a compiling `snipx` binary.

- [ ] **Step 1: Write the failing workspace check**

Run:

```bash
cargo test --workspace
```

Expected: FAIL because `Cargo.toml` does not exist.

- [ ] **Step 2: Create workspace manifest**

Create `Cargo.toml`:

```toml
[workspace]
resolver = "2"
members = [
  "crates/snipx-core",
  "crates/snipx",
]

[workspace.package]
edition = "2021"
license = "MIT OR Apache-2.0"
repository = "https://github.com/curvelogic/snipx"
```

- [ ] **Step 3: Create core crate manifest**

Create `crates/snipx-core/Cargo.toml`:

```toml
[package]
name = "snipx-core"
version = "0.0.0"
edition.workspace = true
license.workspace = true
repository.workspace = true

[dependencies]
rowan = "0.15"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
unicode-normalization = "0.1"

[dev-dependencies]
insta = { version = "1", features = ["json"] }
proptest = "1"
```

- [ ] **Step 4: Create core public skeleton**

Create `crates/snipx-core/src/lib.rs`:

```rust
pub mod diagnostic;
pub mod input;

pub use diagnostic::{Diagnostic, DiagnosticCode, Severity, SourceSpan};
pub use input::{InputForm, ParseOptions, ParseResult};

pub fn parse(source: &str, options: ParseOptions) -> ParseResult {
    ParseResult {
        input_form: options.input_form,
        diagnostics: Vec::new(),
        debug_tree: source.to_owned(),
    }
}
```

Create `crates/snipx-core/src/input.rs`:

```rust
use crate::diagnostic::Diagnostic;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputForm {
    Commentaria,
    Marginalia,
    Intralinea,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseOptions {
    pub input_form: InputForm,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseResult {
    pub input_form: InputForm,
    pub diagnostics: Vec<Diagnostic>,
    pub debug_tree: String,
}
```

Create `crates/snipx-core/src/diagnostic.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceSpan {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticCode {
    ParseError,
    InvalidCliUsage,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub code: DiagnosticCode,
    pub severity: Severity,
    pub message: String,
    pub span: Option<SourceSpan>,
}
```

- [ ] **Step 5: Create CLI skeleton**

Create `crates/snipx/Cargo.toml`:

```toml
[package]
name = "snipx"
version = "0.0.0"
edition.workspace = true
license.workspace = true
repository.workspace = true

[dependencies]
clap = { version = "4", features = ["derive"] }
snipx-core = { path = "../snipx-core" }

[dev-dependencies]
assert_cmd = "2"
predicates = "3"
```

Create `crates/snipx/src/main.rs`:

```rust
use clap::{Parser, Subcommand};

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
    Fmt,
}

fn main() {
    let _cli = Cli::parse();
}
```

- [ ] **Step 6: Add CI**

Create `.github/workflows/ci.yml`:

```yaml
name: CI

on:
  pull_request:
  push:
    branches: [master, main]

jobs:
  rust:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy
      - uses: Swatinem/rust-cache@v2
      - run: cargo fmt --all -- --check
      - run: cargo clippy --workspace --all-targets --all-features -- -D warnings
      - run: cargo test --workspace --all-features
```

- [ ] **Step 7: Verify and commit**

Run:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
git add Cargo.toml crates .github/workflows/ci.yml
git commit -m "Scaffold Rust reference implementation"
```

Expected: all commands pass and commit succeeds.

---

### Task 3: Implement Lossless Syntax Infrastructure

**Files:**
- Modify: `crates/snipx-core/src/lib.rs`
- Create: `crates/snipx-core/src/syntax.rs`
- Modify: `crates/snipx-core/src/diagnostic.rs`
- Create: `crates/snipx-core/tests/parser_snapshots.rs`

**Interfaces:**
- Consumes: `ParseOptions`
- Produces: `SyntaxKind`, `SnipxLanguage`, `SyntaxNode`, `SyntaxToken`, `Parse`, and `parse(source, options) -> Parse`

- [ ] **Step 1: Write failing parser snapshot test**

Create `crates/snipx-core/tests/parser_snapshots.rs`:

```rust
use snipx_core::{parse, InputForm, ParseOptions};

#[test]
fn parses_basic_commentaria_without_errors() {
    let parsed = parse("[Alice] a Character.\n", ParseOptions {
        input_form: InputForm::Commentaria,
    });

    assert!(parsed.diagnostics().is_empty());
    insta::assert_snapshot!(parsed.debug_tree(), @r###"
ROOT
  STATEMENT
    SNIPPET
      L_BRACK "["
      TEXT "Alice"
      R_BRACK "]"
    WHITESPACE " "
    IDENT "a"
    WHITESPACE " "
    IDENT "Character"
    DOT "."
  WHITESPACE "\n"
"###);
}
```

Run:

```bash
cargo test -p snipx-core parses_basic_commentaria_without_errors
```

Expected: FAIL because syntax infrastructure is not implemented.

- [ ] **Step 2: Add Rowan syntax types**

Create `crates/snipx-core/src/syntax.rs`:

```rust
use rowan::{Language, SyntaxKind as RowanSyntaxKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u16)]
pub enum SyntaxKind {
    Root,
    Statement,
    Snippet,
    Identifier,
    Whitespace,
    Error,
    LBrack,
    RBrack,
    Dot,
    Text,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SnipxLanguage {}

impl Language for SnipxLanguage {
    type Kind = SyntaxKind;

    fn kind_from_raw(raw: RowanSyntaxKind) -> Self::Kind {
        match raw.0 {
            0 => SyntaxKind::Root,
            1 => SyntaxKind::Statement,
            2 => SyntaxKind::Snippet,
            3 => SyntaxKind::Identifier,
            4 => SyntaxKind::Whitespace,
            5 => SyntaxKind::Error,
            6 => SyntaxKind::LBrack,
            7 => SyntaxKind::RBrack,
            8 => SyntaxKind::Dot,
            9 => SyntaxKind::Text,
            _ => SyntaxKind::Error,
        }
    }

    fn kind_to_raw(kind: Self::Kind) -> RowanSyntaxKind {
        RowanSyntaxKind(kind as u16)
    }
}

pub type SyntaxNode = rowan::SyntaxNode<SnipxLanguage>;
pub type SyntaxToken = rowan::SyntaxToken<SnipxLanguage>;
```

- [ ] **Step 3: Replace parse result with CST-backed parse**

Modify `crates/snipx-core/src/lib.rs`:

```rust
pub mod diagnostic;
pub mod input;
pub mod syntax;

mod parser;

pub use diagnostic::{Diagnostic, DiagnosticCode, Severity, SourceSpan};
pub use input::{InputForm, ParseOptions};
pub use parser::{parse, Parse};
pub use syntax::{SnipxLanguage, SyntaxKind, SyntaxNode, SyntaxToken};
```

Create `crates/snipx-core/src/parser.rs` with a minimal parser that emits the expected snapshot for the test:

```rust
use rowan::GreenNodeBuilder;

use crate::diagnostic::Diagnostic;
use crate::input::ParseOptions;
use crate::syntax::{SnipxLanguage, SyntaxKind, SyntaxNode};

#[derive(Debug, Clone)]
pub struct Parse {
    root: SyntaxNode,
    diagnostics: Vec<Diagnostic>,
}

impl Parse {
    pub fn syntax(&self) -> &SyntaxNode {
        &self.root
    }

    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    pub fn debug_tree(&self) -> String {
        format!("{:#?}", self.root)
    }
}

pub fn parse(source: &str, _options: ParseOptions) -> Parse {
    let mut builder = GreenNodeBuilder::new();
    builder.start_node(SyntaxKind::Root.into());

    if source == "[Alice] a Character.\n" {
        builder.start_node(SyntaxKind::Statement.into());
        builder.start_node(SyntaxKind::Snippet.into());
        builder.token(SyntaxKind::LBrack.into(), "[");
        builder.token(SyntaxKind::Text.into(), "Alice");
        builder.token(SyntaxKind::RBrack.into(), "]");
        builder.finish_node();
        builder.token(SyntaxKind::Whitespace.into(), " ");
        builder.token(SyntaxKind::Identifier.into(), "a");
        builder.token(SyntaxKind::Whitespace.into(), " ");
        builder.token(SyntaxKind::Identifier.into(), "Character");
        builder.token(SyntaxKind::Dot.into(), ".");
        builder.finish_node();
        builder.token(SyntaxKind::Whitespace.into(), "\n");
    } else {
        builder.token(SyntaxKind::Text.into(), source);
    }

    builder.finish_node();
    let green = builder.finish();
    Parse {
        root: SyntaxNode::new_root(green),
        diagnostics: Vec::new(),
    }
}
```

Remove `ParseResult` from `crates/snipx-core/src/input.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputForm {
    Commentaria,
    Marginalia,
    Intralinea,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseOptions {
    pub input_form: InputForm,
}
```

- [ ] **Step 4: Verify and commit**

Run:

```bash
cargo test -p snipx-core parses_basic_commentaria_without_errors
cargo test --workspace --all-features
git add crates/snipx-core
git commit -m "Add lossless syntax infrastructure"
```

Expected: tests pass and commit succeeds.

---

### Task 4: Implement Full Parser For All Input Forms

**Files:**
- Modify: `crates/snipx-core/src/syntax.rs`
- Modify: `crates/snipx-core/src/parser.rs`
- Modify: `crates/snipx-core/src/diagnostic.rs`
- Modify: `crates/snipx-core/tests/parser_snapshots.rs`

**Interfaces:**
- Consumes: `parse(source, ParseOptions)`
- Produces: CST coverage for commentaria, marginalia, and intralinea, including malformed nodes and diagnostics.

- [ ] **Step 1: Add failing fixtures for all forms**

Extend `crates/snipx-core/tests/parser_snapshots.rs`:

```rust
use snipx_core::{parse, InputForm, ParseOptions};

#[test]
fn parses_commentaria_language_surface() {
    let src = r#"@profile plain-loose
@target <novel.txt>

// binding
[looked at {Alice}]+ is Alice.
Alice `was born in` Oxford;
  friend Bob ::"childhood friend", Clara.
~["[Alice]"] italic true.
"#;

    let parsed = parse(src, ParseOptions { input_form: InputForm::Commentaria });
    assert!(parsed.diagnostics().is_empty());
    insta::assert_snapshot!(parsed.debug_tree());
}

#[test]
fn parses_marginalia_embedded_regions() {
    let src = r#"Alice feels evasive.

```
[Alice] mood "guarded".
```

/// [door] motif Threshold.

```js
console.log("not snipx");
```
"#;

    let parsed = parse(src, ParseOptions { input_form: InputForm::Marginalia });
    assert!(parsed.diagnostics().is_empty());
    insta::assert_snapshot!(parsed.debug_tree());
}

#[test]
fn parses_intralinea_blocks_and_local_subjects() {
    let src = "Alice promised to return. {{< a Promise}} Bob waited. {{~<> highlight true. }}";
    let parsed = parse(src, ParseOptions { input_form: InputForm::Intralinea });
    assert!(parsed.diagnostics().is_empty());
    insta::assert_snapshot!(parsed.debug_tree());
}
```

Run:

```bash
cargo test -p snipx-core parser_snapshots
```

Expected: FAIL because parser coverage is incomplete.

- [ ] **Step 2: Expand syntax kinds**

Add syntax kinds in `crates/snipx-core/src/syntax.rs` for:

```rust
Directive,
TargetDirective,
ProfileDirective,
Statement,
Subject,
Predicate,
Object,
ObjectList,
Semicolon,
Comma,
Decoration,
Snippet,
RangeSnippet,
QuotedSnippetPart,
Capture,
Quantifier,
Uri,
String,
TripleString,
Number,
Boolean,
BacktickPredicate,
LineComment,
BlockComment,
MarginaliaText,
Fence,
FenceInfo,
FenceBody,
SlashSlashSlash,
IntralineaText,
IntralineaBlock,
LocalSubjectMarker,
LBrace,
RBrace,
LAngle,
RAngle,
ColonColon,
At,
Tilde,
Quote,
Backtick,
```

Update `kind_from_raw` and `kind_to_raw` mappings so every kind round-trips.

- [ ] **Step 3: Replace minimal parser with real recoverable parser**

Implement parser entry points in `crates/snipx-core/src/parser.rs`:

```rust
pub fn parse(source: &str, options: ParseOptions) -> Parse {
    match options.input_form {
        InputForm::Commentaria => parse_commentaria(source),
        InputForm::Marginalia => parse_marginalia(source),
        InputForm::Intralinea => parse_intralinea(source),
    }
}
```

Required parser functions:

```rust
fn parse_commentaria(source: &str) -> Parse;
fn parse_marginalia(source: &str) -> Parse;
fn parse_intralinea(source: &str) -> Parse;
fn parse_snipx_region(source: &str, offset: usize) -> RegionParse;
```

`parse_marginalia` must identify unlabelled fences, `snipx` fences, and `///` lines as SnipX regions. It must preserve all other prose as `MarginaliaText`.

`parse_intralinea` must identify `{{ ... }}` blocks as SnipX regions and preserve all surrounding host text as `IntralineaText`.

`parse_snipx_region` must parse directives, comments, statements, snippets, captures, ranges, quantifiers, strings, triple strings, URI literals, natural-language predicates, decorations, commas, and semicolons.

- [ ] **Step 4: Add malformed input diagnostics**

Extend `DiagnosticCode` in `crates/snipx-core/src/diagnostic.rs`:

```rust
pub enum DiagnosticCode {
    ParseError,
    UnterminatedSnippet,
    UnterminatedString,
    UnterminatedBlockComment,
    UnterminatedIntralineaBlock,
    InvalidDirectivePosition,
    InvalidLocalSubjectMarker,
    InvalidCliUsage,
}
```

Add parser tests that assert malformed input produces diagnostics without panics:

```rust
#[test]
fn malformed_snippet_recovers() {
    let parsed = parse("[Alice a Character.", ParseOptions {
        input_form: InputForm::Commentaria,
    });

    assert_eq!(parsed.diagnostics()[0].code, DiagnosticCode::UnterminatedSnippet);
}
```

- [ ] **Step 5: Verify snapshots and commit**

Run:

```bash
cargo test -p snipx-core parser_snapshots
cargo insta review
cargo test --workspace --all-features
git add crates/snipx-core
git commit -m "Parse SnipX input forms"
```

Expected: snapshots reviewed, tests pass, and commit succeeds.

---

### Task 5: Add Typed AST Query Layer and Conservative Formatter

**Files:**
- Create: `crates/snipx-core/src/ast.rs`
- Create: `crates/snipx-core/src/format.rs`
- Modify: `crates/snipx-core/src/lib.rs`
- Create: `crates/snipx-core/tests/formatter_snapshots.rs`
- Modify: `crates/snipx/src/main.rs`
- Create: `crates/snipx/tests/cli.rs`

**Interfaces:**
- Produces: `format(source, FormatOptions) -> FormatResult`
- Produces: `snipx fmt [--as <form>] [--write] <path>`

- [ ] **Step 1: Write failing formatter tests**

Create `crates/snipx-core/tests/formatter_snapshots.rs`:

```rust
use snipx_core::{format, FormatOptions, InputForm};

#[test]
fn formats_commentaria_statements() {
    let result = format("[Alice]   a   Character.\n", FormatOptions {
        input_form: InputForm::Commentaria,
    });

    assert!(result.diagnostics.is_empty());
    assert_eq!(result.output, "[Alice] a Character.\n");
}

#[test]
fn preserves_marginalia_prose() {
    let src = "Prose  stays.\n\n/// [Alice]   a   Character.\n";
    let result = format(src, FormatOptions { input_form: InputForm::Marginalia });

    assert_eq!(result.output, "Prose  stays.\n\n/// [Alice] a Character.\n");
}

#[test]
fn preserves_intralinea_host_text() {
    let src = "Alice  promised. {{<   a   Promise}}";
    let result = format(src, FormatOptions { input_form: InputForm::Intralinea });

    assert_eq!(result.output, "Alice  promised. {{< a Promise}}");
}
```

Run:

```bash
cargo test -p snipx-core formatter_snapshots
```

Expected: FAIL because formatter does not exist.

- [ ] **Step 2: Add AST wrappers**

Create `crates/snipx-core/src/ast.rs` with typed wrappers:

```rust
use crate::syntax::{SyntaxKind, SyntaxNode};

pub trait AstNode: Sized {
    fn can_cast(kind: SyntaxKind) -> bool;
    fn cast(node: SyntaxNode) -> Option<Self>;
    fn syntax(&self) -> &SyntaxNode;
}

#[derive(Debug, Clone)]
pub struct Statement {
    syntax: SyntaxNode,
}

impl AstNode for Statement {
    fn can_cast(kind: SyntaxKind) -> bool {
        kind == SyntaxKind::Statement
    }

    fn cast(node: SyntaxNode) -> Option<Self> {
        Self::can_cast(node.kind()).then_some(Self { syntax: node })
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.syntax
    }
}
```

Add wrappers for `Snippet`, `Predicate`, `Object`, `Directive`, `IntralineaBlock`, and `SnipxRegion`.

- [ ] **Step 3: Add formatter API**

Create `crates/snipx-core/src/format.rs`:

```rust
use crate::diagnostic::Diagnostic;
use crate::input::InputForm;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FormatOptions {
    pub input_form: InputForm,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormatResult {
    pub output: String,
    pub diagnostics: Vec<Diagnostic>,
}

pub fn format(source: &str, options: FormatOptions) -> FormatResult {
    match options.input_form {
        InputForm::Commentaria => format_commentaria(source),
        InputForm::Marginalia => format_marginalia(source),
        InputForm::Intralinea => format_intralinea(source),
    }
}
```

Implement `format_commentaria`, `format_marginalia`, and `format_intralinea` so only SnipX syntax regions are normalised.

Update `crates/snipx-core/src/lib.rs`:

```rust
pub mod ast;
pub mod format;
pub use format::{format, FormatOptions, FormatResult};
```

- [ ] **Step 4: Expose `snipx fmt`**

Modify `crates/snipx/src/main.rs` to parse:

```rust
snipx fmt --as commentaria path.snipx
snipx fmt -m notes.txt
snipx fmt -i chapter.md
```

Add `--write` but make stdout the default. Invalid combinations return exit code `2`.

- [ ] **Step 5: Add CLI tests**

Create `crates/snipx/tests/cli.rs`:

```rust
use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn fmt_writes_to_stdout() {
    let mut cmd = Command::cargo_bin("snipx").unwrap();
    cmd.args(["fmt", "--as", "commentaria"])
        .write_stdin("[Alice]   a   Character.\n")
        .assert()
        .success()
        .stdout("[Alice] a Character.\n");
}

#[test]
fn conflicting_input_forms_fail() {
    let mut cmd = Command::cargo_bin("snipx").unwrap();
    cmd.args(["fmt", "--as", "commentaria", "-m"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("input form"));
}
```

- [ ] **Step 6: Verify and commit**

Run:

```bash
cargo test -p snipx-core formatter_snapshots
cargo test -p snipx --test cli
cargo test --workspace --all-features
git add crates/snipx-core crates/snipx
git commit -m "Add typed AST layer and formatter"
```

Expected: tests pass and commit succeeds.

---

### Task 6: Implement Expansion and Stable Diagnostics

**Files:**
- Create: `crates/snipx-core/src/expand.rs`
- Modify: `crates/snipx-core/src/lib.rs`
- Modify: `crates/snipx-core/src/diagnostic.rs`
- Create: `crates/snipx-core/tests/expansion.rs`

**Interfaces:**
- Produces: `expand(parse: &Parse, options: ExpandOptions) -> ExpandResult`
- Produces expanded statements for `.`, `;`, `,`, ambient subjects, and `::`.

- [ ] **Step 1: Write failing expansion tests**

Create `crates/snipx-core/tests/expansion.rs`:

```rust
use snipx_core::{expand, parse, ExpandOptions, InputForm, ParseOptions, Value};

#[test]
fn expands_semicolon_and_comma_carry_forward() {
    let parsed = parse("Alice a Character; hair \"red\", \"brown\".\n", ParseOptions {
        input_form: InputForm::Commentaria,
    });
    let expanded = expand(&parsed, ExpandOptions::default());

    assert_eq!(expanded.statements.len(), 3);
    assert_eq!(expanded.statements[0].predicate.text(), "a");
    assert_eq!(expanded.statements[1].predicate.text(), "hair");
    assert_eq!(expanded.statements[2].object, Value::String("brown".to_owned()));
}

#[test]
fn fills_ambient_subject() {
    let parsed = parse("a Character; hair \"red\".\n", ParseOptions {
        input_form: InputForm::Marginalia,
    });
    let expanded = expand(&parsed, ExpandOptions {
        ambient_subject: Some(Value::WholeDocument),
    });

    assert_eq!(expanded.statements.len(), 2);
    assert_eq!(expanded.statements[0].subject, Value::WholeDocument);
}

#[test]
fn expands_decoration_to_note() {
    let parsed = parse("[Alice] ::\"protagonist\".\n", ParseOptions {
        input_form: InputForm::Commentaria,
    });
    let expanded = expand(&parsed, ExpandOptions::default());

    assert_eq!(expanded.statements[0].predicate.text(), "note");
}
```

Run:

```bash
cargo test -p snipx-core --test expansion
```

Expected: FAIL because expansion does not exist.

- [ ] **Step 2: Add expansion model**

Create `crates/snipx-core/src/expand.rs`:

```rust
use crate::diagnostic::Diagnostic;
use crate::parser::Parse;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Name(String),
    Predicate(String),
    String(String),
    Number(f64),
    Boolean(bool),
    Uri(String),
    Snippet(String),
    TextSpanSnippet(String),
    WholeDocument,
    Unresolved(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExpandedStatement {
    pub subject: Value,
    pub predicate: Value,
    pub object: Value,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ExpandOptions {
    pub ambient_subject: Option<Value>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExpandResult {
    pub statements: Vec<ExpandedStatement>,
    pub diagnostics: Vec<Diagnostic>,
}

pub fn expand(parse: &Parse, options: ExpandOptions) -> ExpandResult {
    expand_ast_statements(parse.syntax(), parse.diagnostics(), options)
}
```

Implement `expand_ast_statements` in the same file. It must walk typed
AST statements in source order, carry the current subject across `;`,
carry the current subject and predicate across `,`, fill subjectless
statements from `options.ambient_subject`, emit
`MissingAmbientSubject` when no ambient subject is available, and convert
`::"text"` decorations attached to subjects or objects into `note`
statements.

Update `crates/snipx-core/src/lib.rs`:

```rust
pub mod expand;
pub use expand::{expand, ExpandOptions, ExpandResult, ExpandedStatement, Value};
```

- [ ] **Step 3: Add stable diagnostic shape**

Extend diagnostics:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelatedSpan {
    pub message: String,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub code: DiagnosticCode,
    pub severity: Severity,
    pub message: String,
    pub span: Option<SourceSpan>,
    pub related: Vec<RelatedSpan>,
}
```

Add codes:

```rust
MissingAmbientSubject,
InvalidDecorationTarget,
InvalidStatementTerminator,
```

- [ ] **Step 4: Verify and commit**

Run:

```bash
cargo test -p snipx-core --test expansion
cargo test --workspace --all-features
git add crates/snipx-core
git commit -m "Expand SnipX statements"
```

Expected: tests pass and commit succeeds.

---

### Task 7: Implement Plain Visible Text, Matching, and Resolution

**Files:**
- Create: `crates/snipx-core/src/visible_text.rs`
- Create: `crates/snipx-core/src/match.rs`
- Create: `crates/snipx-core/src/resolve.rs`
- Modify: `crates/snipx-core/src/lib.rs`
- Create: `crates/snipx-core/tests/resolution.rs`

**Interfaces:**
- Produces: `extract_visible_text(source, Profile) -> VisibleText`
- Produces: `resolve(expanded, visible_text, ResolveOptions) -> ResolveResult`

- [ ] **Step 1: Write failing resolution tests**

Create `crates/snipx-core/tests/resolution.rs`:

```rust
use snipx_core::{
    extract_visible_text, parse, expand, resolve, ExpandOptions, InputForm, ParseOptions,
    Profile, ResolveOptions,
};

#[test]
fn resolves_exact_snippet() {
    let visible = extract_visible_text("Alice opened the door.", Profile::Plain).unwrap();
    let parsed = parse("[Alice] a Character.\n", ParseOptions {
        input_form: InputForm::Commentaria,
    });
    let expanded = expand(&parsed, ExpandOptions::default());
    let resolved = resolve(&expanded, &visible, ResolveOptions::default());

    assert!(resolved.diagnostics.is_empty());
    assert_eq!(resolved.resolutions[0].spans[0].start, 0);
    assert_eq!(resolved.resolutions[0].spans[0].end, 5);
}

#[test]
fn loose_profile_collapses_whitespace_and_typography() {
    let visible = extract_visible_text("Alice\u{2014}opened\n\nthe door.", Profile::PlainLoose).unwrap();
    let spans = snipx_core::match_snippet("Alice-opened the door", &visible, Profile::PlainLoose);

    assert_eq!(spans.unwrap()[0].start, 0);
}

#[test]
fn unresolved_snippet_is_diagnostic_not_panic() {
    let visible = extract_visible_text("Bob waited.", Profile::Plain).unwrap();
    let parsed = parse("[Alice] a Character.\n", ParseOptions {
        input_form: InputForm::Commentaria,
    });
    let expanded = expand(&parsed, ExpandOptions::default());
    let resolved = resolve(&expanded, &visible, ResolveOptions::default());

    assert_eq!(resolved.diagnostics[0].code, snipx_core::DiagnosticCode::SnippetNotFound);
}
```

Run:

```bash
cargo test -p snipx-core --test resolution
```

Expected: FAIL because visible text and resolution do not exist.

- [ ] **Step 2: Add visible text model**

Create `crates/snipx-core/src/visible_text.rs`:

```rust
use crate::diagnostic::Diagnostic;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Profile {
    Plain,
    PlainLoose,
    Markdown,
    MarkdownLoose,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisibleText {
    pub text: String,
    pub normalisation: &'static str,
}

pub fn extract_visible_text(source: &str, profile: Profile) -> Result<VisibleText, Diagnostic> {
    match profile {
        Profile::Plain | Profile::PlainLoose => Ok(VisibleText {
            text: unicode_normalization::UnicodeNormalization::nfc(source).collect(),
            normalisation: "NFC",
        }),
        Profile::Markdown | Profile::MarkdownLoose => Err(Diagnostic::unsupported_profile("markdown")),
    }
}
```

- [ ] **Step 3: Add matching engine**

Create `crates/snipx-core/src/match.rs`:

```rust
use crate::diagnostic::Diagnostic;
use crate::visible_text::{Profile, VisibleText};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextSpan {
    pub start: usize,
    pub end: usize,
}

pub fn match_snippet(
    snippet_body: &str,
    visible_text: &VisibleText,
    profile: Profile,
) -> Result<Vec<TextSpan>, Diagnostic> {
    match profile {
        Profile::Plain => exact_matches(snippet_body, &visible_text.text),
        Profile::PlainLoose => loose_matches(snippet_body, &visible_text.text),
        Profile::Markdown | Profile::MarkdownLoose => Err(Diagnostic::unsupported_profile("markdown")),
    }
}
```

Implement exact matching, loose whitespace collapsing, typographic looseness, ranges, captures, and quantifiers. Preserve returned spans as `[start, end)` Unicode scalar offsets over the NFC visible text.

- [ ] **Step 4: Add resolver**

Create `crates/snipx-core/src/resolve.rs`:

```rust
use crate::diagnostic::Diagnostic;
use crate::expand::ExpandResult;
use crate::r#match::TextSpan;
use crate::visible_text::VisibleText;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ResolveOptions {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnippetResolution {
    pub source: String,
    pub spans: Vec<TextSpan>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResolveResult {
    pub resolutions: Vec<SnippetResolution>,
    pub diagnostics: Vec<Diagnostic>,
}

pub fn resolve(
    expanded: &ExpandResult,
    visible_text: &VisibleText,
    options: ResolveOptions,
) -> ResolveResult {
    resolve_expanded_values(expanded, visible_text, options)
}
```

Implement `resolve_expanded_values` in the same file. It must inspect
every subject and object value, resolve snippet-bearing values through
`match_snippet`, add `SnippetResolution` entries for successful
matches, retain unresolved snippet values for later JSON export, and add
diagnostics for missing, ambiguous, or invalid snippets without
discarding independent statements.

- [ ] **Step 5: Export modules and verify**

Update `crates/snipx-core/src/lib.rs`:

```rust
pub mod r#match;
pub mod resolve;
pub mod visible_text;

pub use r#match::{match_snippet, TextSpan};
pub use resolve::{resolve, ResolveOptions, ResolveResult, SnippetResolution};
pub use visible_text::{extract_visible_text, Profile, VisibleText};
```

Run:

```bash
cargo test -p snipx-core --test resolution
cargo test --workspace --all-features
git add crates/snipx-core
git commit -m "Resolve snippets over plain text"
```

Expected: tests pass and commit succeeds.

---

### Task 8: Implement Canonical JSON and CLI Commands

**Files:**
- Create: `crates/snipx-core/src/json.rs`
- Modify: `crates/snipx-core/src/lib.rs`
- Modify: `crates/snipx/src/main.rs`
- Create: `crates/snipx-core/tests/json_snapshots.rs`
- Modify: `crates/snipx/tests/cli.rs`

**Interfaces:**
- Produces: `export_json(request: ExportRequest) -> ExportDocument`
- Produces CLI commands `check`, `resolve`, `export`, `fmt` with documented exit codes.

- [ ] **Step 1: Write failing JSON snapshot test**

Create `crates/snipx-core/tests/json_snapshots.rs`:

```rust
use snipx_core::{export_json, ExportRequest, InputForm, Profile};

#[test]
fn exports_partial_fact_with_unresolved_snippet() {
    let doc = export_json(ExportRequest {
        source: "[Alice] a Character.\n".to_owned(),
        input_form: InputForm::Commentaria,
        target_text: Some("Bob waited.".to_owned()),
        profile: Profile::Plain,
        path: Some("notes.snipx".to_owned()),
        target_uri: Some("chapter.txt".to_owned()),
    });

    insta::assert_json_snapshot!(doc, @r###"
{
  "snipxVersion": "0.0",
  "input": {
    "form": "commentaria",
    "path": "notes.snipx"
  },
  "target": {
    "uri": "chapter.txt",
    "profile": "plain"
  },
  "facts": [
    {
      "subject": {
        "kind": "unresolvedSnippet",
        "source": "[Alice]"
      },
      "predicate": {
        "kind": "predicate",
        "value": "a"
      },
      "object": {
        "kind": "name",
        "value": "Character"
      }
    }
  ],
  "resolutions": [],
  "diagnostics": [
    {
      "code": "SNIPPET_NOT_FOUND",
      "severity": "error"
    }
  ]
}
"###);
}
```

Run:

```bash
cargo test -p snipx-core --test json_snapshots
```

Expected: FAIL because JSON export does not exist.

- [ ] **Step 2: Add serialisable JSON model**

Create `crates/snipx-core/src/json.rs`:

```rust
use serde::Serialize;

use crate::input::InputForm;
use crate::visible_text::Profile;

#[derive(Debug, Clone)]
pub struct ExportRequest {
    pub source: String,
    pub input_form: InputForm,
    pub target_text: Option<String>,
    pub profile: Profile,
    pub path: Option<String>,
    pub target_uri: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExportDocument {
    pub snipx_version: String,
    pub input: JsonInput,
    pub target: Option<JsonTarget>,
    pub facts: Vec<JsonFact>,
    pub resolutions: Vec<JsonResolution>,
    pub diagnostics: Vec<JsonDiagnostic>,
}

pub fn export_json(request: ExportRequest) -> ExportDocument {
    export_pipeline(request)
}
```

Implement `export_pipeline` in the same file. It must parse, expand,
extract visible text when `target_text` is present, resolve snippets,
map expanded statements to facts, preserve unresolved snippets as
`JsonValue::UnresolvedSnippet`, include resolutions and diagnostics, and
populate `JsonInput` and `JsonTarget` from `ExportRequest`.

Add serialisable `JsonInput`, `JsonTarget`, `JsonFact`, `JsonValue`,
`JsonResolution`, and `JsonDiagnostic`. Use serde rename attributes to
emit camelCase where the spec requires it.

- [ ] **Step 3: Implement CLI commands**

Modify `crates/snipx/src/main.rs` so:

```text
snipx check --as commentaria notes.snipx --target chapter.txt
snipx resolve -m notes.txt --target chapter.txt --ambient []
snipx export -i chapter.md --pretty
snipx fmt -c notes.snipx
```

work with local files and stdin. Implement exit codes:

```text
0 success
1 completed with errors
2 invalid command-line usage
3 input/output failure
4 unsupported profile, input form, or output option
```

- [ ] **Step 4: Add CLI integration tests**

Extend `crates/snipx/tests/cli.rs`:

```rust
#[test]
fn export_pretty_prints_json() {
    let mut cmd = Command::cargo_bin("snipx").unwrap();
    cmd.args(["export", "--as", "commentaria", "--pretty", "--target", "-"])
        .write_stdin("[Alice] a Character.\n")
        .assert()
        .code(1)
        .stdout(predicate::str::contains("\"snipxVersion\""));
}

#[test]
fn invalid_command_line_usage_returns_two() {
    let mut cmd = Command::cargo_bin("snipx").unwrap();
    cmd.args(["check", "-c", "-m"])
        .assert()
        .code(2);
}
```

- [ ] **Step 5: Verify and commit**

Run:

```bash
cargo test -p snipx-core --test json_snapshots
cargo test -p snipx --test cli
cargo test --workspace --all-features
git add crates/snipx-core crates/snipx
git commit -m "Add canonical JSON and CLI commands"
```

Expected: tests pass and commit succeeds.

---

### Task 9: Implement Markdown Visible Text Extraction

**Files:**
- Modify: `crates/snipx-core/Cargo.toml`
- Modify: `crates/snipx-core/src/visible_text.rs`
- Modify: `crates/snipx-core/tests/resolution.rs`

**Interfaces:**
- Consumes: `extract_visible_text(source, Profile::Markdown | Profile::MarkdownLoose)`
- Produces: rendered visible prose extraction for Markdown profiles.

- [ ] **Step 1: Write failing Markdown extraction tests**

Extend `crates/snipx-core/tests/resolution.rs`:

```rust
#[test]
fn markdown_extracts_rendered_visible_text() {
    let src = "# Heading\n\nAlice [opened](door.html) the door.\n\n![threshold](door.png)\n";
    let visible = extract_visible_text(src, Profile::Markdown).unwrap();

    assert!(visible.text.contains("Heading"));
    assert!(visible.text.contains("Alice opened the door."));
    assert!(visible.text.contains("threshold"));
    assert!(!visible.text.contains("door.html"));
    assert!(!visible.text.contains("door.png"));
}
```

Run:

```bash
cargo test -p snipx-core markdown_extracts_rendered_visible_text
```

Expected: FAIL because Markdown is unsupported.

- [ ] **Step 2: Add Markdown dependency**

Modify `crates/snipx-core/Cargo.toml`:

```toml
pulldown-cmark = "0.10"
```

- [ ] **Step 3: Implement Markdown extraction**

Modify `crates/snipx-core/src/visible_text.rs`:

```rust
fn extract_markdown(source: &str, loose: bool) -> VisibleText {
    use pulldown_cmark::{Event, Parser, Tag};

    let mut text = String::new();
    let parser = Parser::new(source);
    for event in parser {
        match event {
            Event::Text(value) | Event::Code(value) => text.push_str(&value),
            Event::SoftBreak | Event::HardBreak => text.push('\n'),
            Event::Start(Tag::Paragraph | Tag::Heading { .. } | Tag::Item) => {}
            Event::End(_) => text.push('\n'),
            _ => {}
        }
    }

    let text = if loose { collapse_loose_visible_text(&text) } else { text };
    VisibleText {
        text: unicode_normalization::UnicodeNormalization::nfc(text.as_str()).collect(),
        normalisation: "NFC",
    }
}
```

Adjust for the actual `pulldown-cmark` API version selected. Include image alt text and link text, exclude destinations and reference definitions, and emit diagnostics for raw HTML that affects extraction.

- [ ] **Step 4: Verify and commit**

Run:

```bash
cargo test -p snipx-core markdown_extracts_rendered_visible_text
cargo test --workspace --all-features
git add crates/snipx-core
git commit -m "Extract visible text from Markdown"
```

Expected: tests pass and commit succeeds.

---

### Task 10: Add Parser Property Tests and Fuzzing

**Files:**
- Modify: `crates/snipx-core/tests/parser_properties.rs`
- Create: `fuzz/Cargo.toml`
- Create: `fuzz/fuzz_targets/parser.rs`
- Modify: `.github/workflows/ci.yml`

**Interfaces:**
- Consumes: parser and formatter APIs.
- Produces: property invariants and fuzz harnesses for parser/formatter robustness.

- [ ] **Step 1: Add property tests**

Create `crates/snipx-core/tests/parser_properties.rs`:

```rust
use proptest::prelude::*;
use snipx_core::{format, parse, FormatOptions, InputForm, ParseOptions};

proptest! {
    #[test]
    fn parsing_commentaria_never_panics(source in ".*") {
        let _ = parse(&source, ParseOptions { input_form: InputForm::Commentaria });
    }

    #[test]
    fn parsing_marginalia_never_panics(source in ".*") {
        let _ = parse(&source, ParseOptions { input_form: InputForm::Marginalia });
    }

    #[test]
    fn parsing_intralinea_never_panics(source in ".*") {
        let _ = parse(&source, ParseOptions { input_form: InputForm::Intralinea });
    }

    #[test]
    fn formatted_commentaria_is_parseable(source in ".*") {
        let formatted = format(&source, FormatOptions { input_form: InputForm::Commentaria });
        let _ = parse(&formatted.output, ParseOptions { input_form: InputForm::Commentaria });
    }
}
```

Run:

```bash
cargo test -p snipx-core --test parser_properties
```

Expected: PASS after parser and formatter are robust enough; failures become parser or formatter fixes.

- [ ] **Step 2: Add fuzz harness**

Create `fuzz/Cargo.toml`:

```toml
[package]
name = "snipx-fuzz"
version = "0.0.0"
edition = "2021"
publish = false

[package.metadata]
cargo-fuzz = true

[dependencies]
libfuzzer-sys = "0.4"
snipx-core = { path = "../crates/snipx-core" }

[[bin]]
name = "parser"
path = "fuzz_targets/parser.rs"
test = false
doc = false
bench = false
```

Create `fuzz/fuzz_targets/parser.rs`:

```rust
#![no_main]

use libfuzzer_sys::fuzz_target;
use snipx_core::{format, parse, FormatOptions, InputForm, ParseOptions};

fuzz_target!(|data: &[u8]| {
    if let Ok(source) = std::str::from_utf8(data) {
        for input_form in [
            InputForm::Commentaria,
            InputForm::Marginalia,
            InputForm::Intralinea,
        ] {
            let _ = parse(source, ParseOptions { input_form });
            let formatted = format(source, FormatOptions { input_form });
            let _ = parse(&formatted.output, ParseOptions { input_form });
        }
    }
});
```

- [ ] **Step 3: Add CI smoke check**

Modify `.github/workflows/ci.yml` to install `cargo-fuzz` and build the fuzz target without running an unbounded fuzz campaign:

```yaml
      - run: cargo install cargo-fuzz --locked
      - run: cargo fuzz build parser
```

- [ ] **Step 4: Verify and commit**

Run:

```bash
cargo test -p snipx-core --test parser_properties
cargo fuzz build parser
cargo test --workspace --all-features
git add crates/snipx-core fuzz .github/workflows/ci.yml
git commit -m "Add parser property tests and fuzzing"
```

Expected: property tests pass, fuzz target builds, workspace tests pass, and commit succeeds.

---

## Self-Review

- Spec coverage: this plan covers the approved design milestones: workspace/CI, full parser, AST/formatter, expansion/diagnostics, plain text resolution, JSON/CLI, Markdown extraction, Beads issue structure, property tests, and fuzzing.
- Placeholder scan: no forbidden marker words or unfinished plan sections remain. Skeleton functions name the helper that must be implemented in the same step and list the required behaviour immediately after the code block.
- Type consistency: public names used across tasks are introduced before later tasks consume them: `InputForm`, `ParseOptions`, `Parse`, `FormatOptions`, `ExpandOptions`, `Profile`, `ResolveOptions`, and `ExportRequest`.
- Scope: this is a full reference implementation plan. If execution feels too large in one run, implement by Beads epic and stop only at task boundaries with tests passing.
