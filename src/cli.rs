use crate::error::AppError;

/// The synopsis repeated with every parse failure so the user always learns
/// which forms are valid after a mistake.
///
/// The type `&str` is a string *slice*: a borrowed view over characters that
/// someone else owns. A string literal is `'static`, meaning it lives in the
/// read-only data of the binary for the whole process, so the constant needs
/// no allocation and no lifetime bookkeeping.
const USAGE: &str = "usage: todo <add <text>|search <text>|list|done <id>|delete <id>|quit>";

/// A fully validated invocation of the binary.
///
/// `#[derive(...)]` is an attribute macro: it asks the compiler to generate
/// the listed trait implementations instead of writing them by hand. `Debug`
/// enables `{:?}` printing, `PartialEq`/`Eq` enable `assert_eq!` in the
/// tests. Serde's `Serialize`/`Deserialize` on `Todo` are ordinary derive
/// macros too — attributes can produce code, not just metadata.
#[derive(Debug, PartialEq, Eq)]
pub enum Command {
    /// Persist a new todo carrying the given text.
    Add(String),
    /// Report every todo whose text contains the given text.
    Search(String),
    /// Print every todo together with a completion summary.
    List,
    /// Flag the todo with this identifier as completed.
    Done(u32),
    /// Drop the todo with this identifier from the store.
    Delete(u32),
    /// Leave the interactive session, or bid farewell in one-shot mode.
    Quit,
}

/// Builds a [`Command`] from the arguments that follow the program name.
///
/// Parsing is all-or-nothing: every failure surfaces as [`AppError::Usage`]
/// carrying a one-line explanation above the synopsis, so the caller never
/// observes partially interpreted input.
///
/// Mechanically, `args.into_iter()` consumes the vector and yields owned
/// `String`s; the `let mut args` rebinding is required because `next()` takes
/// `&mut self`. The scrutinee `args.next().as_deref()` produces an
/// `Option<&str>` — `as_deref` borrows the inner `String` as a slice — which
/// lets arms match against string literals like `Some("list")` instead of
/// constructing equal `String`s. The temporary `Option` lives for the whole
/// `match`, yet `args` is only mutably borrowed during the call to `next()`,
/// so later arms may keep draining the same iterator.
///
/// `Some(other) => ...` binds the leftover verb, and `{other}` in `format!`
/// reads that binding by name: a format string may capture any variable in
/// scope rather than using positional `{}` placeholders.
pub fn parse(args: Vec<String>) -> Result<Command, AppError> {
    let mut args = args.into_iter();
    match args.next().as_deref() {
        None => Err(usage("a command is required")),
        Some("list") => Ok(Command::List),
        // The two verbs that take free text share one implementation below,
        // so neither arm contains anything but the command it builds. In
        // both, `?` sits *inside* `Ok(...)`: `joined_text` may fail, and `?`
        // returns that error from `parse` before `Ok` is ever constructed.
        Some("add") => Ok(Command::Add(joined_text("add", args)?)),
        Some("search") => Ok(Command::Search(joined_text("search", args)?)),
        // The `?` sits *inside* `Ok(...)`: `parse_id` may fail, and `?`
        // returns that error from `parse` before `Ok` is ever constructed.
        Some("done") => Ok(Command::Done(parse_id(args.collect())?)),
        Some("delete") => Ok(Command::Delete(parse_id(args.collect())?)),
        // `|` inside a pattern is an or-pattern: either spelling is Quit.
        Some("quit" | "exit") => Ok(Command::Quit),
        Some(other) => Err(usage(&format!("unknown command '{other}'"))),
    }
}

/// Rebuilds the remaining words into one trimmed, non-empty string.
///
/// `add` and `search` both take a phrase rather than a single token, and
/// both must reject a phrase that turns out to be blank, so the work lives
/// here once instead of twice. Naming the verb is what lets the failure say
/// which command was incomplete.
///
/// The signature deserves two readings. `args: impl Iterator<Item = String>`
/// is *argument-position `impl Trait`*, shorthand for a private generic
/// parameter: the caller may hand in any iterator of owned strings, and each
/// call site is compiled with the type it actually passed. Here that type is
/// the `std::vec::IntoIter` left behind after the verb itself was consumed.
///
/// `collect::<Vec<_>>()` is a *turbofish*: `::<>` supplies the type
/// argument that `collect` cannot infer by itself, since it is generic over
/// every collection type, while `_` lets the element type be deduced from
/// the iterator. `join` then rebuilds one string — so `todo add buy milk`
/// works without shell quoting, and the whitespace between words collapses
/// to a single space — and `trim` strips any leading or leftover blanks.
fn joined_text(verb: &str, args: impl Iterator<Item = String>) -> Result<String, AppError> {
    let text = args.collect::<Vec<_>>().join(" ").trim().to_string();
    if text.is_empty() {
        return Err(usage(&format!("{verb} requires non-empty text")));
    }
    Ok(text)
}

/// Extracts the single numeric operand required by `done` and `delete`.
///
/// Missing, surplus and non-numeric arguments are all rejected here rather
/// than being silently coerced, which keeps bad ids from reaching the store.
///
/// The scrutinee is a tuple of two `Option<String>`s built by calling `next()`
/// twice, and the patterns destructure it position by position; the `_` in
/// `(None, _)` means "whatever follows, I don't care about it". In the middle
/// arm, `.parse::<u32>()` is another turbofish selecting the destination
/// integer type, yielding `Result<u32, ParseIntError>`; `map_err(|_| ...)`
/// then swaps that library error for friendlier text, and the closure simply
/// discards its argument by naming it `_`.
fn parse_id(args: Vec<String>) -> Result<u32, AppError> {
    // Convert the vector into an iterator that yields owned `String` values.
    // `mut` allows us to call `.next()` to consume items one by one.
    let mut args = args.into_iter();

    // Match on a tuple containing the 1st and 2nd arguments from the iterator:
    match (args.next(), args.next()) {
        // Case 1: No arguments provided (the iterator returned `None` immediately).
        // Returns an error explaining that an ID is required.
        (None, _) => Err(usage("a numeric id is required")),

        // Case 2: Exactly one argument was provided (`Some(raw)` for 1st, `None` for 2nd).
        (Some(raw), None) => raw
            // Attempt to parse the raw string slice into an unsigned 32-bit integer (`u32`).
            .parse::<u32>()
            // If parsing fails (e.g. "abc" or negative numbers), map the ParseIntError
            // into our custom `AppError` via `usage()`.
            .map_err(|_| usage(&format!("'{raw}' is not a valid id"))),

        // Case 3: More than one argument was provided (`Some` for both 1st and 2nd).
        // Rejects extra positional arguments.
        (Some(_), Some(_)) => Err(usage("only one id may be given")),
    }
}
/// Wraps a one-line explanation in the standard usage error.
///
/// `{detail}` and `{USAGE}` are inline format captures of the parameter and
/// the module constant, and `\n` puts the synopsis on its own line.
fn usage(detail: &str) -> AppError {
    AppError::Usage(format!("{detail}\n{USAGE}"))
}

#[cfg(test)]
mod tests;
