use rustyline::{
    error::ReadlineError,
    DefaultEditor,
};

use novum::{
    runtime::{ControlFlow, Value},
    Interpreter, Lexer, Parser,
};

use std::{
    env,
    fs,
};

const VERSION: &str = env!("CARGO_PKG_VERSION");

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
        let mut options = Self::default();

        for arg in env::args().skip(1) {
            match arg.as_str() {
                "--version" | "-V" => {
                    return Ok(Command::Version);
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

                "help" | "--help" | "-h" => {
                    return Ok(Command::Help);
                }

                _ if arg.starts_with('-') => {
                    return Err(format!("unknown option: {arg}"));
                }

                _ => {
                    if options.file.is_some() {
                        return Err(
                            "only one input file is allowed".into()
                        );
                    }

                    options.file = Some(arg);
                }
            }
        }

        Ok(Command::Run(options))
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
    help             Show REPL commands
    quit, exit       Exit the REPL

KEYS:
    ↑ / ↓            Navigate command history
    ← / →            Move the cursor
    Home / End       Move to line boundaries
    Ctrl-C           Cancel current input
    Ctrl-D           Exit the REPL
"
        );
    }
}

fn main() {
    let command = match Options::parse() {
        Ok(command) => command,

        Err(message) => {
            eprintln!("error: {message}");
            eprintln!("try 'novum --help' for usage information");
            std::process::exit(1);
        }
    };

    match command {
        Command::Version => {
            println!("novum v{VERSION}");
        }

        Command::Help => {
            Options::print_help();
        }

        Command::Run(options) => {
            let mut interpreter = Interpreter::new();

            if let Some(path) = options.file {
                if let Err(error) = run_file(
                    &mut interpreter,
                    &path,
                    options.display_lexer,
                    options.display_parser,
                ) {
                    eprintln!("{error}");
                    std::process::exit(1);
                }
            } else {
                println!("novum v{VERSION}\n");

                repl(
                    &mut interpreter,
                    options.display_lexer,
                    options.display_parser,
                );
            }
        }
    }
}

fn run_file(
    interpreter: &mut Interpreter,
    path: &str,
    display_lexer: bool,
    display_parser: bool,
) -> Result<(), String> {
    let source = fs::read_to_string(path)
        .map_err(|e| format!("failed to read '{path}': {e}"))?;

    run(
        interpreter,
        &source,
        display_lexer,
        display_parser,
        None,
    );

    Ok(())
}

fn run(
    interpreter: &mut Interpreter,
    source: &str,
    display_lexer: bool,
    display_parser: bool,
    line_index: Option<usize>,
) {
    let mut lexer = Lexer::new(source);

    let tokens = match lexer.lex() {
        Ok(tokens) => tokens,

        Err(error) => {
            error.display(source);
            return;
        }
    };

    if display_lexer {
        println!("\nTokens:\n{tokens:#?}");
    }

    let mut parser = Parser::new(tokens);

    let program = match parser.parse() {
        Ok(program) => program,

        Err(error) => {
            error.display(source);
            return;
        }
    };

    if display_parser {
        println!("\nAST:\n{program:#?}");
    }

    match interpreter.eval_program(&program) {
        Ok(ControlFlow::Value(value))
            if value != Value::Unit =>
        {
            print_result(value, line_index);
        }

        Ok(_) => {}

        Err(error) => {
            error.display(source);
        }
    }
}

fn print_result(
    value: Value,
    line_index: Option<usize>,
) {
    let output = value.to_string();

    let prefix = match line_index {
        Some(index) => format!("[{index}] >> "),
        None => ">> ".to_string(),
    };

    let indent = " ".repeat(prefix.len());

    let mut lines = output.lines();

    if let Some(first) = lines.next() {
        println!("{prefix}{first}");

        for line in lines {
            println!("{indent}{line}");
        }
    }
}

fn repl(
    interpreter: &mut Interpreter,
    display_lexer: bool,
    display_parser: bool,
) {
    let mut editor = match DefaultEditor::new() {
        Ok(editor) => editor,

        Err(error) => {
            eprintln!("failed to initialize REPL: {error}");
            return;
        }
    };

    let mut line_index = 0usize;

    loop {
        let prompt = format!("\n[{line_index}] << ");

        match editor.readline(&prompt) {
            Ok(line) => {
                let command = line.trim();

                if command.is_empty() {
                    continue;
                }

                match command {
                    "quit" | "exit" => {
                        println!("\nBye!");
                        break;
                    }

                    "help" => {
                        print_repl_help();
                        continue;
                    }

                    _ => {}
                }

                if let Err(error) =
                    editor.add_history_entry(&line)
                {
                    eprintln!(
                        "warning: failed to save history entry: {error}"
                    );
                }

                run(
                    interpreter,
                    &line,
                    display_lexer,
                    display_parser,
                    Some(line_index),
                );

                line_index += 1;
            }

            Err(ReadlineError::Interrupted) => {
                println!("^C");
            }

            Err(ReadlineError::Eof) => {
                println!("\nBye!");
                break;
            }

            Err(error) => {
                eprintln!("REPL error: {error}");
                break;
            }
        }
    }
}

fn print_repl_help() {
    println!(
        "\
REPL commands:

    help             Show this help
    quit, exit       Exit the REPL

Keyboard:

    ↑ / ↓            Command history
    ← / →            Cursor movement
    Home / End       Line boundaries
    Ctrl-C           Cancel input
    Ctrl-D           Exit
"
    );
}