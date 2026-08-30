use novum::{
    runtime::{Value},
    syntax::TokenKind,
    vm::{Vm, Compiler},
    Lexer, Parser,
};

use reedline::{
    default_emacs_keybindings,
    EditCommand,
    Emacs,
    FileBackedHistory,
    Highlighter,
    KeyCode,
    KeyModifiers,
    Prompt,
    Reedline,
    ReedlineEvent,
    Signal,
    StyledText,
    ValidationResult,
    Validator,
};

use nu_ansi_term::{Color, Style};

use std::{
    borrow::Cow,
    rc::Rc,
    cell::RefCell,
    env,
    fs,
    path::{
        PathBuf,
    },
};

const VERSION: &str = env!("CARGO_PKG_VERSION");

// ============================================================
// CLI
// ============================================================

#[derive(Debug, Default)]
struct Options {
    display_lexer: bool,
    display_parser: bool,
    file: Option<String>,
}

enum Command {
    Run(Options),
    Help,
    Version,
}

impl Options {
    fn parse() -> Result<Command, String> {
        let mut options =
            Self::default();

        let args =
            env::args().skip(1);

        for arg in args
        {
            match arg.as_str() {
                "--version" | "-V" => {
                    return Ok(
                        Command::Version
                    );
                }

                "-l" | "--lexer" => {
                    options.display_lexer = true;
                }

                "-p" | "--parser" => {
                    options.display_parser = true;
                }

                "-a" | "--all" => {
                    options.display_lexer = true;
                    options.display_parser = true;
                }

                "help"
                | "--help"
                | "-h" => {
                    return Ok(
                        Command::Help
                    );
                }

                _ if arg.starts_with('-') => {
                    return Err(
                        format!(
                            "unknown option: {arg}"
                        )
                    );
                }

                _ => {
                    if options.file.is_some() {
                        return Err(
                            "only one input file is allowed"
                                .into()
                        );
                    }

                    options.file =
                        Some(arg);
                }
            }
        }

        Ok(
            Command::Run(
                options
            )
        )
    }

    fn print_help() {
        println!(
            "\
novum v{VERSION}

USAGE:
    novum [OPTIONS] [FILE]

OPTIONS:
    -l, --lexer      Show lexer output
    -p, --parser     Show parser output
    -a, --all        Show lexer and parser output
    -h, --help       Show this help message
    -V, --version    Show version information

REPL:
    novum            Start the Novum VM REPL
    help             Show REPL commands
    quit, exit       Exit the REPL

KEYS:
    ↑ / ↓            Navigate command history
    ← / →            Move the cursor
    Home / End       Move to line boundaries
    Shift+Enter      Insert a new line
    Ctrl+Enter       Insert a new line
    Ctrl-C           Cancel input
    Ctrl-D           Exit
"
        );
    }
}

// ============================================================
// Main
// ============================================================

fn main() {
    let command =
        match Options::parse() {
            Ok(command) =>
                command,

            Err(message) => {
                eprintln!(
                    "error: {message}"
                );

                eprintln!(
                    "try 'novum --help' for usage information"
                );

                std::process::exit(1);
            }
        };

    match command {
        Command::Version => {
            println!(
                "novum v{VERSION}"
            );
        }

        Command::Help => {
            Options::print_help();
        }

        Command::Run(options) => {
            match options.file {
                Some(path) => {
                    run_file(
                        path,
                        options.display_lexer,
                        options.display_parser,
                    );
                }

                None => {
                    repl(
                        options.display_lexer,
                        options.display_parser,
                    );
                }
            }
        }
    }
}

// ============================================================
// Execution
// ============================================================

fn run_file(
    path: String,
    display_lexer: bool,
    display_parser: bool,
) {
    let source =
        match fs::read_to_string(
            &path
        ) {
            Ok(source) =>
                source,

            Err(error) => {
                eprintln!(
                    "failed to read '{}': {}",
                    path,
                    error
                );

                std::process::exit(1);
            }
        };

    let source_path =
        match fs::canonicalize(&path) {
            Ok(path) => path,

            Err(error) => {
                eprintln!(
                    "failed to resolve '{}': {}",
                    path,
                    error
                );

                std::process::exit(1);
            }
        };

    let tokens =
        match Lexer::new(&source).lex() {
            Ok(tokens) =>
                tokens,

            Err(error) => {
                error.display(
                    &source
                );

                std::process::exit(1);
            }
        };

    if display_lexer {
        println!(
            "\nTokens:\n{tokens:#?}"
        );
    }

    let mut parser =
        Parser::new(tokens);

    let program =
        match parser.parse() {
            Ok(program) =>
                program,

            Err(error) => {
                error.display(
                    &source
                );

                std::process::exit(1);
            }
        };

    if display_parser {
        println!(
            "\nAST:\n{program:#?}"
        );
    }

    let chunk =
        match Compiler::new()
            .compile(&program)
        {
            Ok(chunk) =>
                Rc::new(chunk),

            Err(error) => {
                error.display(
                    &source
                );

                std::process::exit(1);
            }
        };

    let module =
        Rc::new(
            RefCell::new(
                novum::runtime::Module::new(
                    "<main>"
                )
            )
        );

    let mut vm =
        Vm::new();

    match vm.run_with_module_and_path(
        chunk,
        module,
        Some(&source_path),
    ) {
        Ok(Value::Unit) => {}

        Ok(value) => {
            println!("{value}");
        }

        Err(error) => {
            error.display(&source);
            std::process::exit(1);
        }
    }
}

// ============================================================
// REPL
// ============================================================

fn repl(
    display_lexer: bool,
    display_parser: bool,
) {
    let history =
        match FileBackedHistory::with_file(
            1000,
            history_path(),
        ) {
            Ok(history) => {
                Box::new(history)
            }

            Err(error) => {
                eprintln!(
                    "warning: failed to initialize history: {error}"
                );

                Box::new(
                    FileBackedHistory::default()
                )
            }
        };

    let keybindings =
        configure_keybindings();

    let edit_mode =
        Box::new(
            Emacs::new(
                keybindings
            )
        );

    let mut editor =
        Reedline::create()
            .with_history(history)
            .with_edit_mode(edit_mode)
            .with_validator(
                Box::new(
                    NovumValidator
                )
            )
            .with_highlighter(
                Box::new(
                    NovumHighlighter
                )
            )
            .use_kitty_keyboard_enhancement(
                true
            )
            .with_ansi_colors(
                true
            );

    let mut compiler =
        Compiler::new();

    let mut vm =
        Vm::new();

    let mut line_index =
        0usize;

    loop {
        println!();

        let prompt =
            NovumPrompt::new(
                line_index
            );

        match editor.read_line(
            &prompt
        ) {
            Ok(
                Signal::Success(line)
            ) => {
                let command =
                    line.trim();

                if command.is_empty() {
                    continue;
                }

                match command {
                    "quit"
                    | "exit" => {
                        println!(
                            "\nBye!"
                        );

                        break;
                    }

                    "help" => {
                        print_repl_help();
                        continue;
                    }

                    _ => {}
                }

                run_repl_line(
                    &mut compiler,
                    &mut vm,
                    &line,
                    display_lexer,
                    display_parser,
                    line_index,
                );

                line_index += 1;
            }

            Ok(
                Signal::CtrlC
            ) => {
                println!("^C");
            }

            Ok(
                Signal::CtrlD
            ) => {
                println!(
                    "\nBye!"
                );

                break;
            }

            Ok(signal) => {
                eprintln!(
                    "REPL event: {signal:?}"
                );
            }

            Err(error) => {
                eprintln!(
                    "REPL error: {error}"
                );

                break;
            }
        }
    }
}

fn run_repl_line(
    compiler: &mut Compiler,
    vm: &mut Vm,
    source: &str,
    display_lexer: bool,
    display_parser: bool,
    line_index: usize,
) {
    let mut lexer =
        Lexer::new(source);

    let tokens =
        match lexer.lex() {
            Ok(tokens) =>
                tokens,

            Err(error) => {
                error.display(
                    source
                );

                return;
            }
        };

    if display_lexer {
        println!(
            "\nTokens:\n{tokens:#?}"
        );
    }

    let mut parser =
        Parser::new(tokens);

    let program =
        match parser.parse() {
            Ok(program) =>
                program,

            Err(error) => {
                error.display(
                    source
                );

                return;
            }
        };

    if display_parser {
        println!(
            "\nAST:\n{program:#?}"
        );
    }

    let chunk =
        match compiler
            .compile_program(
                &program
            )
        {
            Ok(chunk) =>
                Rc::new(chunk),

            Err(error) => {
                error.display(
                    source
                );

                return;
            }
        };

    match vm.run_repl(
        chunk
    ) {
        Ok(value)
            if value != Value::Unit =>
        {
            print_result(
                value,
                Some(line_index),
                true,
            );
        }

        Ok(_) => {}

        Err(error) => {
            error.display(
                source
            );
        }
    }
}

// ============================================================
// UI Styles
// ============================================================

struct NovumUi;

impl NovumUi {
    // --------------------------------------------------------
    // Prompt
    // --------------------------------------------------------

    fn prompt_style() -> Style {
        Style::new()
            .fg(Color::Blue)
            .bold()
    }

    fn repl_command_style() -> Style {
        Style::new()
            .fg(Color::Blue)
            .bold()
    }

    // --------------------------------------------------------
    // Syntax
    // --------------------------------------------------------

    fn keyword_style() -> Style {
        Style::new()
            .fg(Color::Purple)
            .bold()
    }

    fn literal_style() -> Style {
        Style::new()
            .fg(Color::Cyan)
    }

    fn string_style() -> Style {
        Style::new()
            .fg(Color::Green)
    }

    fn bool_style() -> Style {
        Style::new()
            .fg(Color::Yellow)
            .bold()
    }

    fn null_style() -> Style {
        Style::new()
            .fg(Color::LightYellow)
            .bold()
    }

    fn paren_style() -> Style {
        Style::new()
            .fg(Color::Blue)
    }

    fn bracket_style() -> Style {
        Style::new()
            .fg(Color::Cyan)
    }

    fn brace_style() -> Style {
        Style::new()
            .fg(Color::Magenta)
    }

    fn default_style() -> Style {
        Style::default()
    }

    fn style_for_token(
        kind: &TokenKind,
    ) -> Style {
        match kind {
            // ------------------------------------------------
            // Literals
            // ------------------------------------------------

            TokenKind::Int(_)
            | TokenKind::Float(_) => {
                Self::literal_style()
            }

            TokenKind::Str(_) => {
                Self::string_style()
            }

            TokenKind::Bool(_) => {
                Self::bool_style()
            }

            TokenKind::Null => {
                Self::null_style()
            }

            // ------------------------------------------------
            // Keywords
            // ------------------------------------------------

            TokenKind::If
            | TokenKind::Else
            | TokenKind::For
            | TokenKind::While
            | TokenKind::Break
            | TokenKind::Continue
            | TokenKind::Return
            | TokenKind::In
            | TokenKind::As
            | TokenKind::Let
            | TokenKind::Pub
            | TokenKind::Match
            | TokenKind::Class
            | TokenKind::Struct
            | TokenKind::Enum
            | TokenKind::Import => {
                Self::keyword_style()
            }

            // ------------------------------------------------
            // Brackets
            // ------------------------------------------------

            TokenKind::LParen
            | TokenKind::RParen => {
                Self::paren_style()
            }

            TokenKind::Pipe => {
                Self::paren_style()
            }

            TokenKind::LBracket
            | TokenKind::RBracket => {
                Self::bracket_style()
            }

            TokenKind::LBrace
            | TokenKind::RBrace => {
                Self::brace_style()
            }

            // ------------------------------------------------
            // Everything else
            // ------------------------------------------------

            _ => Self::default_style(),
        }
    }

    // --------------------------------------------------------
    // Prompt text
    // --------------------------------------------------------

    fn styled_prompt(
        text: &str,
    ) -> String {
        Self::prompt_style()
            .paint(text)
            .to_string()
    }

    // --------------------------------------------------------
    // Source highlighting
    // --------------------------------------------------------

    fn highlight_source(
        source: &str,
    ) -> StyledText {
        let mut styled = StyledText::new();

        // REPL commands are not Novum syntax.
        let command = source.trim();

        if matches!(
            command,
            "quit" | "exit" | "help"
        ) {
            styled.push((
                Self::repl_command_style(),
                source.to_string(),
            ));

            return styled;
        }

        let mut lexer = Lexer::new(source);

        let tokens = match lexer.lex() {
            Ok(tokens) => tokens,

            // Incomplete input is expected while editing.
            // Do not make the highlighter itself fail.
            Err(_) => {
                styled.push((
                    Self::default_style(),
                    source.to_string(),
                ));

                return styled;
            }
        };

        let mut last = 0usize;

        for token in tokens {
            if matches!(
                token.kind,
                TokenKind::Eof
            ) {
                break;
            }

            let start = token.span.start;
            let end = token.span.end;

            // Safety against malformed spans.
            if start > source.len()
                || end > source.len()
                || start > end
            {
                styled.push((
                    Self::default_style(),
                    source.to_string(),
                ));

                return styled;
            }

            // Preserve whitespace / gaps.
            if start > last {
                styled.push((
                    Self::default_style(),
                    source[last..start].to_string(),
                ));
            }

            styled.push((
                Self::style_for_token(
                    &token.kind
                ),
                source[start..end].to_string(),
            ));

            last = end;
        }

        // Preserve trailing text.
        if last < source.len() {
            styled.push((
                Self::default_style(),
                source[last..].to_string(),
            ));
        }

        styled
    }

    // --------------------------------------------------------
    // Result rendering
    // --------------------------------------------------------

    fn render_result(
        value: &Value,
        line_index: Option<usize>,
        colorize: bool,
    ) -> String {
        let output = value.to_string();

        let prefix = match line_index {
            Some(index) => {
                format!("[{index}] >> ")
            }

            None => {
                ">> ".to_string()
            }
        };

        let indent = " ".repeat(prefix.len());

        let mut styled = StyledText::new();

        let mut lines = output.lines();

        let Some(first) = lines.next() else {
            return String::new();
        };

        // Prefix.
        if colorize {
            styled.push((
                Self::prompt_style(),
                prefix,
            ));
        } else {
            styled.push((
                Self::default_style(),
                prefix,
            ));
        }

        // First line.
        if colorize {
            Self::append_highlighted(
                &mut styled,
                first,
            );
        } else {
            styled.push((
                Self::default_style(),
                first.to_string(),
            ));
        }

        // Remaining lines.
        for line in lines {
            styled.push((
                Self::default_style(),
                format!("\n{indent}"),
            ));

            if colorize {
                Self::append_highlighted(
                    &mut styled,
                    line,
                );
            } else {
                styled.push((
                    Self::default_style(),
                    line.to_string(),
                ));
            }
        }

        styled.render_simple()
    }

    fn append_highlighted(
        target: &mut StyledText,
        source: &str,
    ) {
        let highlighted =
            Self::highlight_source(source);

        for segment in highlighted.buffer {
            target.push(segment);
        }
    }
}

// ============================================================
// Output
// ============================================================

fn print_result(
    value: Value,
    line_index: Option<usize>,
    colorize: bool,
) {
    let output = NovumUi::render_result(
        &value,
        line_index,
        colorize,
    );

    if !output.is_empty() {
        println!("{output}");
    }
}

// ============================================================
// Highlighter
// ============================================================

struct NovumHighlighter;

impl Highlighter for NovumHighlighter {
    fn highlight(
        &self,
        line: &str,
        _cursor: usize,
    ) -> StyledText {
        NovumUi::highlight_source(line)
    }
}

// ============================================================
// Prompt
// ============================================================

struct NovumPrompt {
    index: usize,
}

impl NovumPrompt {
    fn new(index: usize) -> Self {
        Self { index }
    }

    fn prefix(&self) -> String {
        format!("[{}] << ", self.index)
    }

    fn prefix_width(&self) -> usize {
        self.prefix().len()
    }
}

impl Prompt for NovumPrompt {
    fn render_prompt_left(
        &self,
    ) -> Cow<'_, str> {
        Cow::Owned(
            NovumUi::styled_prompt(
                &format!("[{}] ", self.index)
            )
        )
    }

    fn render_prompt_right(
        &self,
    ) -> Cow<'_, str> {
        Cow::Borrowed("")
    }

    fn render_prompt_indicator(
        &self,
        _edit_mode: reedline::PromptEditMode,
    ) -> Cow<'_, str> {
        Cow::Owned(
            NovumUi::styled_prompt("<< ")
        )
    }

    fn render_prompt_multiline_indicator(
        &self,
    ) -> Cow<'_, str> {
        Cow::Owned(
            " ".repeat(
                self.prefix_width()
            )
        )
    }

    fn render_prompt_history_search_indicator(
        &self,
        _history_search: reedline::PromptHistorySearch,
    ) -> Cow<'_, str> {
        Cow::Owned(
            NovumUi::styled_prompt(
                "(reverse-search) "
            )
        )
    }
}

// ============================================================
// Validator
// ============================================================

struct NovumValidator;

impl Validator for NovumValidator {
    fn validate(
        &self,
        line: &str,
    ) -> ValidationResult {
        if line.trim().is_empty() {
            return ValidationResult::Complete;
        }

        if has_unclosed_delimiter(line) {
            ValidationResult::Incomplete
        } else {
            ValidationResult::Complete
        }
    }
}

fn has_unclosed_delimiter(
    source: &str,
) -> bool {
    let mut stack = Vec::new();

    let mut in_string = false;
    let mut escaped = false;

    for ch in source.chars() {
        if in_string {
            if escaped {
                escaped = false;
                continue;
            }

            match ch {
                '\\' => {
                    escaped = true;
                }

                '"' => {
                    in_string = false;
                }

                _ => {}
            }

            continue;
        }

        match ch {
            '"' => {
                in_string = true;
            }

            '(' | '[' | '{' => {
                stack.push(ch);
            }

            ')' | ']' | '}' => {
                let Some(open) =
                    stack.pop()
                else {
                    return false;
                };

                let matched = matches!(
                    (open, ch),
                    ('(', ')')
                        | ('[', ']')
                        | ('{', '}')
                );

                if !matched {
                    return false;
                }
            }

            _ => {}
        }
    }

    in_string || !stack.is_empty()
}

// ============================================================
// Keybindings
// ============================================================

fn configure_keybindings()
    -> reedline::Keybindings
{
    let mut keybindings =
        default_emacs_keybindings();

    // --------------------------------------------------------
    // Single quotes
    // --------------------------------------------------------

    keybindings.add_binding(
        KeyModifiers::NONE,
        KeyCode::Char('\''),
        ReedlineEvent::Edit(vec![
            EditCommand::InsertString(
                "''".into()
            ),
            EditCommand::MoveLeft {
                select: false,
            },
        ]),
    );

    // --------------------------------------------------------
    // Double quotes
    // --------------------------------------------------------

    keybindings.add_binding(
        KeyModifiers::NONE,
        KeyCode::Char('"'),
        ReedlineEvent::Edit(vec![
            EditCommand::InsertString(
                "\"\"".into()
            ),
            EditCommand::MoveLeft {
                select: false,
            },
        ]),
    );

    // --------------------------------------------------------
    // Parentheses
    // --------------------------------------------------------

    keybindings.add_binding(
        KeyModifiers::NONE,
        KeyCode::Char('('),
        ReedlineEvent::Edit(vec![
            EditCommand::InsertString(
                "()".into()
            ),
            EditCommand::MoveLeft {
                select: false,
            },
        ]),
    );

    // --------------------------------------------------------
    // Square brackets
    // --------------------------------------------------------

    keybindings.add_binding(
        KeyModifiers::NONE,
        KeyCode::Char('['),
        ReedlineEvent::Edit(vec![
            EditCommand::InsertString(
                "[]".into()
            ),
            EditCommand::MoveLeft {
                select: false,
            },
        ]),
    );

    // --------------------------------------------------------
    // Curly brackets
    // --------------------------------------------------------

    keybindings.add_binding(
        KeyModifiers::NONE,
        KeyCode::Char('{'),
        ReedlineEvent::Edit(vec![
            EditCommand::InsertString(
                "{}".into()
            ),
            EditCommand::MoveLeft {
                select: false,
            },
        ]),
    );

    // --------------------------------------------------------
    // Closures
    // --------------------------------------------------------

    keybindings.add_binding(
        KeyModifiers::NONE,
        KeyCode::Char('|'),
        ReedlineEvent::Edit(vec![
            EditCommand::InsertString(
                "||".into()
            ),
            EditCommand::MoveLeft {
                select: false,
            },
        ]),
    );

    // --------------------------------------------------------
    // Explicit newlines
    // --------------------------------------------------------

    keybindings.add_binding(
        KeyModifiers::SHIFT,
        KeyCode::Enter,
        ReedlineEvent::Edit(vec![
            EditCommand::InsertNewline,
        ]),
    );

    keybindings.add_binding(
        KeyModifiers::CONTROL,
        KeyCode::Enter,
        ReedlineEvent::Edit(vec![
            EditCommand::InsertNewline,
        ]),
    );

    // --------------------------------------------------------
    // Indentation
    // --------------------------------------------------------
    keybindings.add_binding(
        KeyModifiers::NONE,
        KeyCode::Tab,
        ReedlineEvent::Edit(vec![
            EditCommand::InsertString("    ".into()),
        ]),
    );

    keybindings
}

// ============================================================
// History
// ============================================================

fn history_path() -> PathBuf {
    if let Some(home) =
        env::var_os("HOME")
    {
        return PathBuf::from(home)
            .join(".novum_history");
    }

    if let Some(home) =
        env::var_os("USERPROFILE")
    {
        return PathBuf::from(home)
            .join(".novum_history");
    }

    PathBuf::from(".novum_history")
}

// ============================================================
// REPL Help
// ============================================================

fn print_repl_help() {
    println!(
        "\
REPL commands:

    help             Show this help
    quit, exit       Exit the REPL

Keyboard:

    ↑ / ↓            Command history
    ← / →            Move the cursor
    Home / End       Move to line boundaries
    Shift+Enter      Insert a new line
    Ctrl+Enter       Insert a new line
    Ctrl-C           Cancel input
    Ctrl-D           Exit
"
    );
}