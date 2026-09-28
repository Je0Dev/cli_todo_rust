use std::fmt::{self, Display, Formatter};

/// The single error type that can reach the top level of the program.
///
/// It unifies the two independent failure domains of the app, filesystem and
/// JSON access (`Io`, `Json`) with command-line misuse (`Usage`, `NotFound`),
/// so every function in the crate can return `Result<_, AppError>` and let
/// `?` convert the underlying errors through the `From` impls below.
///
/// Syntactically this is a tagged union: the enum's name is the set of
/// variants, and each variant optionally carries a payload of its own type,
/// as in `Usage(String)`. The payload can only be recovered by matching, so
/// no caller can observe an error without deciding what to do with it.
#[derive(Debug)]
pub enum AppError {
    /// A filesystem operation failed for a reason unrelated to JSON.
    Io(std::io::Error),
    /// The on-disk document could not be parsed or serialized.
    Json(serde_json::Error),
    /// The invocation was malformed; the payload is the message for the user.
    Usage(String),
    /// No todo in the store carries the requested identifier.
    NotFound(u32),
}

impl Display for AppError {
    /// Renders the error as a single line suitable for `stderr`.
    ///
    /// `impl Display for AppError` is how Rust attaches a trait
    /// implementation: the trait's required method is restated with this
    /// type in place of `Self`. Its signature, `fn fmt(&self, f: &mut
    /// Formatter<'_>) -> fmt::Result`, borrows the error immutably and hands
    /// back a *formatter handle* that the `write!` macros write into — they
    /// are macros rather than functions so they can forward the format
    /// string at compile time. The `'_` is an anonymous lifetime: the handle
    /// is only valid for the duration of the call, and the name is elided
    /// because nothing ever needs to refer to it. `f.write_str(message)` is
    /// the allocation-free way to emit a string that is already complete.
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Io(e) => write!(f, "i/o error: {e}"),
            AppError::Json(e) => write!(f, "the data file is not valid todo data: {e}"),
            AppError::Usage(message) => f.write_str(message),
            AppError::NotFound(id) => write!(f, "no todo with id {id}"),
        }
    }
}

impl std::error::Error for AppError {
    /// Exposes the wrapped source so callers can walk the causal chain.
    ///
    /// The return type `Option<&(dyn std::error::Error + 'static)>` is a
    /// trait object behind a shared reference: `dyn` says "some concrete
    /// type this call knows only through the trait", the parentheses group
    /// that bound together with the reference, and `'static` requires the
    /// underlying error to outlive this call. Both payload-carrying variants
    /// return `Some`, the payload-free ones `None`, so `source()` never
    /// invents a cause.
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            AppError::Io(e) => Some(e),
            AppError::Json(e) => Some(e),
            AppError::Usage(_) | AppError::NotFound(_) => None,
        }
    }
}

impl From<std::io::Error> for AppError {
    /// Promotes an `io::Error` so `?` works directly on file operations.
    ///
    /// `impl From<X> for Y` declares an infallible conversion from `X` to
    /// `Y`, and it is exactly the contract the `?` operator consumes: when
    /// an expression yields `Result<_, X>` inside a function returning
    /// `Result<_, Y>`, `?` calls `From::from` on the error before returning
    /// it. Add this impl and every `fs::...?` in the crate compiles without
    /// a hand-written match.
    fn from(e: std::io::Error) -> Self {
        AppError::Io(e)
    }
}

impl From<serde_json::Error> for AppError {
    /// Promotes a `serde_json::Error` so `?` works directly on (de)serialization.
    ///
    /// The same `From`/`?` contract as the impl above, for the other library
    /// the crate talks to; two small impls are what let the rest of the code
    /// use `?` uniformly instead of matching on foreign error types.
    fn from(e: serde_json::Error) -> Self {
        AppError::Json(e)
    }
}
