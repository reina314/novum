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

#[derive(Debug, Default)]
struct Options {
    display_lexer: bool,
    display_parser: bool,
    file: Option<String>,
}

impl Options {
    fn parse() -> Result<Self, String> {
        let mut options = Self::default();

        for arg in env::args().skip(1) {
            match arg.as_str() {
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
                    Self::print_help();
                    std::process::exit(0);
                }

                _ if arg.starts_with('-') => {
                    return Err(format!("unknown option: {arg}"));
                }

                _ => {
                    if options.file.is_some() {
                        return Err("only one input file is allowed".into());
                    }

                    options.file = Some(arg);
                }
            }
        }

        Ok(options)
    }

    fn print_help() {
        println!(
            "\
novum v{}

USAGE:
    novum [OPTIONS] [FILE]

OPTIONS:
    -l, --lexer      Show lexer output
    -p, --parser     Show parser output
    -a, --all        Show lexer and parser output
    -h, --help       Show this help message

REPL:
    help             Show REPL commands
    quit, exit       Exit the REPL

KEYS:
    ↑ / ↓            Navigate command history
    ← / →            Move the cursor
    Home / End       Move to line boundaries
    Ctrl-C           Cancel current input
    Ctrl-D           Exit the REPL
",
            env!("CARGO_PKG_VERSION")
        );
    }
}

fn main() {
    let options = match Options::parse() {
        Ok(options) => options,

        Err(message) => {
            eprintln!("error: {message}");
            eprintln!("try 'novum --help' for usage information");
            std::process::exit(1);
        }
    };

    println!("novum v{}\n", env!("CARGO_PKG_VERSION"));

    let mut interpreter = Interpreter::new();

    match options.file {
        Some(path) => {
            if let Err(error) = run_file(
                &mut interpreter,
                &path,
                options.display_lexer,
                options.display_parser,
            ) {
                eprintln!("{error}");
                std::process::exit(1);
            }
        }

        None => {
            repl(
                &mut interpreter,
                options.display_lexer,
                options.display_parser,
            );
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
        Ok(ControlFlow::Value(value)) if value != Value::Unit => {
            if let Some(index) = line_index {
                println!("[{index}] >> {value}");
            } else {
                println!(">> {value}");
            }
        }

        Ok(_) => {}

        Err(error) => {
            error.display(source);
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

                // Store the command only after accepting it as input.
                if let Err(error) = editor.add_history_entry(line.as_str()) {
                    eprintln!("warning: failed to save history entry: {error}");
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
                continue;
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