use std::fs;
use std::path::PathBuf;

use crate::error::AppError;
use crate::store::Todo;

/// Location of the JSON document, resolved against the current directory so
/// the data lives beside the project that owns it.
pub(crate) const STORE: &str = "todos.json";

/// A place todos live between runs.
///
/// `trait Persistence { ... }` declares method *signatures* with no bodies:
/// it states what any storage must be able to do, never how. The bodies
/// arrive separately in each `impl Persistence for SomeType` block, and the
/// compiler checks at that site that every promised method exists with
/// exactly these types — a missing or mismatched method fails to compile
/// where the implementation is written, not where it is used.
///
/// Writing code against this trait instead of against a concrete type is
/// what buys the application its interchangeable backends: the command
/// engine below never learns whether the todos it loads came from a file or
/// from memory.
///
/// The receivers deserve a close look. `&self` and `&mut self` are just the
/// ordinary reference forms applied to the implicit first parameter that
/// every method receives, and the choice is a promise to the caller: `load`
/// only reads, so it asks for a shared borrow and may run while other
/// readers coexist, whereas `save` replaces the contents and therefore
/// demands exclusive access for its duration. The borrow checker turns those
/// promises into guarantees — no `save` can run while anyone else holds a
/// reference into the same store.
pub trait Persistence {
    /// Returns every todo the backend currently holds.
    ///
    /// A backend that has nothing yet must answer with an empty store rather
    /// than an error, so callers need no special case for a first run.
    fn load(&self) -> Result<Vec<Todo>, AppError>;

    /// Replaces the backend's contents with `todos`.
    ///
    /// The whole document is written in one call: there is no partial state
    /// in which half of the store has been saved.
    fn save(&mut self, todos: &[Todo]) -> Result<(), AppError>;
}

/// Storage backed by a single JSON document on disk.
///
/// This is the backend the application uses in normal operation; the field
/// is private so that every instance is guaranteed to name a document,
/// which is the only invariant the type needs.
pub struct JsonFile {
    /// The document this instance reads from and writes to.
    path: PathBuf,
}

impl JsonFile {
    /// Prepares `path` as the document to use, without touching the disk.
    ///
    /// The parameter is spelled `impl Into<PathBuf>` rather than `PathBuf`:
    /// this is *argument-position `impl Trait`*, shorthand for a private
    /// generic parameter, so each call site instantiates `new` with whatever
    /// type the caller passed. The implied bound `Into<PathBuf>` is met by
    /// `"todos.json"` (std converts `&str`), by a `&Path`, and by a `PathBuf`
    /// itself through the reflexive `impl<T> From<T> for T`. The conversion
    /// happens once, in the body, where `.into()` picks the target type from
    /// the field it initializes.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        JsonFile { path: path.into() }
    }
}

impl Persistence for JsonFile {
    /// Reads and parses the document.
    ///
    /// A missing file is not an error: it is how a fresh checkout greets its
    /// first run, so it maps to an empty store. A document that exists but
    /// cannot be deserialized is reported instead of being replaced, which
    /// protects the user's data from being silently discarded.
    ///
    /// The `match` below is Rust's general branching form: it inspects the
    /// scrutinee's *shape* rather than a boolean, and each arm may end with
    /// an `if` — a *guard* — that refines when the arm applies. Thus
    /// `Err(error) if error.kind() == ...NotFound` catches only a missing
    /// file, while the plain `Err(error)` arm beneath it catches every other
    /// I/O failure; arm order matters, because the first match wins. `?`
    /// appears *inside* the `Ok(...)` arm: a parse failure short-circuits
    /// out of `load`, but a successful parse is re-wrapped in `Ok` by the
    /// arm itself. Finally `error.into()` converts `io::Error` into
    /// `AppError` using the `From` impl — the target type is known from this
    /// method's return type, so no annotation is needed.
    ///
    /// `&self.path` hands over a borrow rather than the `PathBuf` itself,
    /// because `read_to_string` only needs to look at the path; a blanket
    /// `AsRef<Path>` impl for references lets a borrowed path be passed
    /// wherever an owned one would be accepted.
    fn load(&self) -> Result<Vec<Todo>, AppError> {
        match fs::read_to_string(&self.path) {
            Ok(document) => Ok(serde_json::from_str(&document)?),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
            Err(error) => Err(error.into()),
        }
    }

    /// Serializes `todos` and writes the document in one call.
    ///
    /// Serialization happens before the file is touched, so a serialization
    /// failure can never leave a half-written store behind. `to_string_pretty`
    /// borrows its argument — the `&[Todo]` the trait promises — and the
    /// vector stays owned by the caller; `fs::write` then creates or
    /// truncates the file in one call, which is what makes the save atomic
    /// enough for a single-user CLI.
    ///
    /// The receiver is `&mut self` because the trait promises exclusive
    /// access, even though writing a file happens to leave this value
    /// unchanged. An implementation may be *more* permissive than the
    /// signature; it can never be less.
    fn save(&mut self, todos: &[Todo]) -> Result<(), AppError> {
        let document = serde_json::to_string_pretty(todos)?;
        fs::write(&self.path, document)?;
        Ok(())
    }
}

/// Storage that never touches the filesystem.
///
/// Its purpose is testing: command execution can be exercised end to end
/// without creating, cleaning up or racing over a temporary file, and
/// without ever reading the developer's real `todos.json`. The `#[cfg(test)]`
/// below applies that reasoning to the compiler as well — the type is only
/// compiled into test builds, so a release binary containing no test can
/// never be accused of carrying an unused backend.
///
/// `#[derive(Default)]` asks the compiler to build the trait impl that gives
/// an empty store — every field type (`Vec<Todo>`) already provides its own
/// default — which is why `new` below can simply delegate instead of
/// restating the construction.
#[cfg(test)]
#[derive(Default)]
pub struct Memory {
    /// The todos written by the most recent `save`, if any.
    todos: Vec<Todo>,
}

#[cfg(test)]
impl Memory {
    /// Creates an empty in-memory store.
    pub fn new() -> Self {
        Self::default()
    }
}

#[cfg(test)]
impl Persistence for Memory {
    /// Hands back a copy of the stored todos.
    ///
    /// The body is infallible yet the signature still returns `Result`: the
    /// trait fixes one shape for every backend, and callers must not have to
    /// care which one they were given. `Ok(...)` merely wraps the clone.
    fn load(&self) -> Result<Vec<Todo>, AppError> {
        Ok(self.todos.clone())
    }

    /// Replaces the stored todos with a copy of `todos`.
    ///
    /// `todos.to_vec()` copies a `&[Todo]` into an owned `Vec` because the
    /// store must outlive the borrow of the caller's vector; the same clone
    /// is what makes every later `load` independent of the original.
    fn save(&mut self, todos: &[Todo]) -> Result<(), AppError> {
        self.todos = todos.to_vec();
        Ok(())
    }
}

/// Makes a mutable reference to any storage a storage in its own right.
///
/// This is a *blanket impl*: rather than naming one concrete type it covers
/// a whole family — every `&mut P` whose `P` can store todos. Its body is
/// pure delegation, walking `self: &mut &mut P` down two levels with `**`
/// and handing the result back to `P`'s own implementation, so the work is
/// done exactly once.
///
/// The `?Sized` in the bound is what this impl exists for. Type parameters
/// are `Sized` — that is, one known, finite size — unless told otherwise,
/// and `dyn Persistence` (the trait object type) has no such size: its whole
/// purpose is to hide which concrete type is behind the reference. Writing
/// `?Sized` lifts the implicit bound and lets `P = dyn Persistence`, which
/// is precisely the substitution that makes `&mut dyn Persistence` satisfy
/// `Persistence`. Generic code such as `execute` can then receive an erased
/// storage simply by being handed a re-borrow of one.
///
/// Std does the same for its own traits — `impl<W: Write + ?Sized> Write
/// for &mut W` is the identical shape — so this is the idiomatic way to let
/// references stand in for the things they point at.
impl<P: Persistence + ?Sized> Persistence for &mut P {
    fn load(&self) -> Result<Vec<Todo>, AppError> {
        (**self).load()
    }

    fn save(&mut self, todos: &[Todo]) -> Result<(), AppError> {
        (**self).save(todos)
    }
}

#[cfg(test)]
mod tests;
