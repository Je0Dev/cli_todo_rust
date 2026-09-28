mod cli;
mod error;
mod persist;
mod repl;
mod store;
mod view;

#[cfg(test)]
mod tests;

use std::env;
use std::process;

use cli::Command;
use error::AppError;
use persist::{JsonFile, Persistence, STORE};
use view::View;

/// Reports a failed run on `stderr` and exits with a non-zero status.
///
/// `main` itself stays infallible so the error type only has to be rendered
/// once, here, instead of being threaded through the signature.
///
/// Each `mod name;` declares that `src/name.rs` is a child module of this,
/// the crate root, and the `use` statements import the handful of names
/// spelled out in full below. Paths such as `repl::run()` are resolved from
/// the root, so they need no `crate::` prefix inside the root itself.
fn main() {
    // `env::args()` is an iterator of owned `String`s; `skip(1)` drops the
    // program path, which always occupies element zero, and the annotated
    // `collect()` turns the rest into the `Vec<String>` that `one_shot`
    // expects. Without the `: Vec<String>` annotation the compiler could not
    // know which collection to build.
    let args: Vec<String> = env::args().skip(1).collect();
    // One storage value is created here and lent to whichever path runs: the
    // concrete type `JsonFile` is chosen once, and both `repl::run` and
    // `one_shot` receive it through a generic parameter, so each is compiled
    // with that type baked in. Nothing below names `JsonFile` again.
    let mut storage = JsonFile::new(STORE);
    // The `if` is an expression, so it yields the `Result` produced by
    // whichever branch runs — the interactive loop when no arguments were
    // given, the one-shot path otherwise. `&mut storage` is a re-borrow, not
    // a move: the loan ends when the branch returns and `storage` itself
    // stays owned by `main`.
    let outcome = if args.is_empty() {
        repl::run(&mut storage)
    } else {
        one_shot(&mut storage, args)
    };
    // `if let Err(error) = outcome` is a single-arm `match`: the body runs
    // only when the value is an `Err`, binding its payload to `error`.
    // Falling through means the run succeeded, so `main` just returns.
    if let Err(error) = outcome {
        eprintln!("error: {error}");
        process::exit(1);
    }
}

/// Runs the single command given on the command line and prints its report.
///
/// The `<S: Persistence>` in the signature declares a *generic function*:
/// `S` is a type parameter standing for the storage, and the `: Persistence`
/// is its *trait bound*, the requirement every type substituted for `S` must
/// meet. `main` passes a `JsonFile`, so the compiler instantiates this
/// function with `S = JsonFile` and checks the bound once at that call site.
/// This is *static dispatch*: the concrete type is known at compile time, so
/// the call to `storage.load()` compiles to a direct call with no runtime
/// lookup at all.
///
/// Both `?` operators can only exist because this function returns
/// `Result`: each unwraps an `Ok` and, on an `Err`, returns that error from
/// `one_shot` immediately, so `println!` is reached only on success. Read
/// the nested call from the inside out — `cli::parse(args)?` must succeed
/// before `execute` is ever called.
fn one_shot<S: Persistence>(storage: &mut S, args: Vec<String>) -> Result<(), AppError> {
    let report = execute(storage, cli::parse(args)?)?;
    println!("{report}");
    Ok(())
}

/// Loads the store through `storage`, applies `command`, persists the
/// outcome, and returns the text the caller should print.
///
/// `S: Persistence` is the same generic parameter and trait bound seen on
/// `one_shot`, and it is what decouples this function from any particular
/// backend: the tests substitute `Memory` and never create a file, while
/// production substitutes `JsonFile`, and neither call site changes a line
/// of this body. Compilation repeats the body once per substituted type —
/// *monomorphization* — which trades a little binary size for abstraction
/// that costs nothing at run time.
///
/// The `match` is exhaustive: the compiler rejects this function if any
/// variant of `Command` lacks an arm, so adding a variant later forces every
/// site to reconsider it. Arms are expressions, each producing the `String`
/// bound to `report`; those needing two statements are written as blocks,
/// and the block's final expression is the arm's value.
///
/// Patterns like `Command::Add(text)` destructure the variant, moving its
/// payload out of the command, so `store::add(&mut todos, text)` takes that
/// `String` by value and the command cannot be replayed afterwards.
/// `&mut todos` lends the vector mutably: while that loan lives this
/// function may not read or move `todos` itself, which is precisely the
/// guarantee that prevents aliasing. `&todos` in the `List` arm relies on
/// deref coercion — `&Vec<Todo>` widens automatically to the `&[Todo]` that
/// `View::all` accepts — and the view it builds borrows `todos` only for as
/// long as the `to_string()` call that renders it.
///
/// The `Search` arm is the one place two borrows of `todos` overlap: the
/// `Vec<&Todo>` returned by `matching` keeps a shared loan open while
/// `View::matched` wraps it, and the loan ends at the semicolon when the
/// rendered `String` — which owns its own bytes — is all that remains. Both
/// loans are shared, and `todos` is only ever mutated in other arms, so
/// nothing here conflicts.
///
/// Persisting after a read-only `list` is deliberate: the on-disk state
/// always mirrors the in-memory one, and there is exactly one place where
/// the document is written. `storage.load()` needs only `&self` and
/// `storage.save(&todos)` needs `&mut self`, so the shared borrow that
/// produced `todos` must end before the exclusive one begins — it does, at
/// the end of the first line.
pub(crate) fn execute<S: Persistence>(
    storage: &mut S,
    command: Command,
) -> Result<String, AppError> {
    let mut todos = storage.load()?;
    let report = match command {
        Command::List => View::all(&todos).to_string(),
        Command::Search(query) => {
            let matches = store::matching(&todos, &query);
            View::matched(matches).to_string()
        }
        Command::Add(text) => format!("added todo {}", store::add(&mut todos, text)),
        Command::Done(id) => {
            store::complete(&mut todos, id)?;
            format!("marked todo {id} as done")
        }
        Command::Delete(id) => {
            store::delete(&mut todos, id)?;
            format!("deleted todo {id}")
        }
        Command::Quit => "bye".to_string(),
    };
    storage.save(&todos)?;
    Ok(report)
}
