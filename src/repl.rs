use std::io::{self, Write};

use crate::cli::{self, Command};
use crate::error::AppError;
use crate::execute;
use crate::persist::Persistence;

/// Runs the interactive session until the user quits or closes stdin.
///
/// `S: Persistence` threads the storage parameter through from `main`, so
/// the whole session — every command it executes — talks to one backend
/// chosen before the loop starts. The bound is plain `Sized` here, unlike
/// anything accepting a trait object, because `main` only ever lends a
/// concrete `JsonFile` to this function.
///
/// Unlike one-shot mode, a bad command does not end the program: the error is
/// printed and the prompt returns, because a typo while typing by hand is
/// routine rather than exceptional. Only a failure of stdin or stdout itself
/// propagates out of the loop and terminates the process.
///
/// The loop is written with `loop { ... }` plus explicit `return`/`continue`
/// rather than a `for`, since the number of iterations is not known in
/// advance — the session ends either when the user types `quit` or when the
/// input stream reaches end-of-file.
pub fn run<S: Persistence>(storage: &mut S) -> Result<(), AppError> {
    println!("interactive todo — type a command, quit to leave");
    let stdin = io::stdin();
    loop {
        // A prompt carries no newline, and Rust's stdout is line-buffered,
        // so `print!` alone would leave `> ` unsent until the next newline.
        // `flush()` pushes the buffer out; it returns `io::Result<()>`, and
        // `?` converts that error through the `From<io::Error>` impl.
        print!("> ");
        io::stdout().flush()?;
        // `read_line` appends one line (newline included) and returns the
        // number of bytes read. `Ok(0)` means end-of-file: the pipe or the
        // terminal was closed, so the session is over.
        let mut line = String::new();
        if stdin.read_line(&mut line)? == 0 {
            println!();
            return Ok(());
        }
        // `split_whitespace()` yields `&str` slices without allocating, so
        // each one is turned into an owned `String` by `String::from` — a
        // function item passed straight to `map`, which the compiler turns
        // into the closure `|word| String::from(word)`.
        let words: Vec<String> = line.split_whitespace().map(String::from).collect();
        if words.is_empty() {
            continue;
        }
        // `matches!` expands to a `match` returning `true`/`false`; because
        // its pattern `Ok(Command::Quit)` binds nothing, no move occurs and
        // `parsed` can still be consumed by `and_then` two lines below.
        let parsed = cli::parse(words);
        let should_quit = matches!(parsed, Ok(Command::Quit));
        // `and_then` chains the two fallible steps — parse already happened,
        // so this applies `execute` to the `Ok` value and passes any `Err`
        // through untouched, flattening `Result<Command>` into `Result<String>`.
        // `execute` takes two arguments, so it is named inside a closure that
        // supplies the second one: the closure captures `storage` by mutable
        // reference (a re-borrow, not a move), and the borrow lasts only as
        // long as the `match` whose scrutinee is evaluating.
        match parsed.and_then(|command| execute(storage, command)) {
            Ok(report) => println!("{report}"),
            Err(error) => eprintln!("error: {error}"),
        }
        // The farewell for `quit` was printed by the normal report path, so
        // the loop's only remaining job is to stop.
        if should_quit {
            return Ok(());
        }
    }
}
