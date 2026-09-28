use serde::{Deserialize, Serialize};

use crate::error::AppError;

/// One unit of work tracked by the application.
///
/// The struct is shared by the parser, the store and the renderer, so every
/// field is public: there is no invariant that a caller could break, and the
/// serde attributes are the only place where the on-disk shape is decided.
///
/// `#[derive(Serialize, Deserialize)]` generates the code that walks each
/// field by name and writes or reads it as JSON, so `serde_json` never needs
/// to be told the layout by hand. The field types (`u32`, `String`, `bool`)
/// are all themselves serializable, which is a requirement the derive
/// checks at compile time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Todo {
    /// Stable identifier assigned at creation; the operand of `done`/`delete`.
    pub id: u32,
    /// Description exactly as the user entered it.
    pub text: String,
    /// Whether the todo has been completed.
    pub done: bool,
}

/// Appends a todo carrying a fresh identifier and returns that identifier.
///
/// The id is one greater than the largest id currently in the store, so ids
/// are unique among live todos. Numbers belonging to deleted todos may be
/// handed out again: the store keeps no history of retired ids.
///
/// `todos.iter().map(|todo| todo.id).max()` is a lazy adapter chain: `iter`
/// borrows the vector and yields `&Todo`, `map` projects each element onto
/// its id — the closure body is an expression, so no `return` is needed —
/// and `max` consumes the iterator into an `Option<u32>`, which is `None`
/// for an empty store. `unwrap_or(0)` handles that `None` without any risk
/// of panicking, unlike `unwrap`.
pub fn add(todos: &mut Vec<Todo>, text: String) -> u32 {
    let id = todos.iter().map(|todo| todo.id).max().unwrap_or(0) + 1;
    todos.push(Todo {
        id,
        text,
        done: false,
    });
    id
}

/// Flags the todo with `id` as completed.
///
/// Returns [`AppError::NotFound`] instead of a boolean so that a typo in the
/// command line surfaces as an error message rather than silent success.
///
/// The signature deserves a close look. `[Todo]` is a *slice*: a pointer
/// paired with a length, with no capacity, so it cannot grow or shrink.
/// Writing `&mut [Todo]` rather than `&mut Vec<Todo>` states "I only need to
/// mutate elements that already exist", and deref coercion lets a caller pass
/// `&mut todos` for a `Vec<Todo>` without saying anything extra — the `Vec`
/// is borrowed as its underlying slice automatically.
///
/// The chain then goes `iter_mut()` (yielding `&mut Todo`) → `find` (an
/// `Option<&mut Todo>`) → `ok_or(...)`, which converts that option into a
/// `Result` by eagerly building the error, so there is no `match` at all.
/// `?` either unwraps it or returns it from this function, and the final
/// `todo.done = true` writes through the mutable borrow — legal only because
/// the vector was lent to us mutably and nothing else can be using it.
pub fn complete(todos: &mut [Todo], id: u32) -> Result<(), AppError> {
    // 1. Iterate mutably through the slice to locate the matching item.
    let todo = todos
        .iter_mut() // Creates an iterator yielding mutable references (`&mut Todo`).
        .find(|todo| todo.id == id) // Returns `Option<&mut Todo>` for the first item matching the ID condition.
        .ok_or(AppError::NotFound(id))?; // Converts `Option::None` into `Err(AppError::NotFound(id))` and unwraps `Some`.

    // 2. Mutate the `done` field of the retrieved `&mut Todo` reference to `true`.
    todo.done = true;

    // 3. Return `Ok(())` indicating successful completion without returning any additional data.
    Ok(())
}
/// Removes the todo with `id`, keeping the remaining order intact.
///
/// `position` is the index-returning sibling of `find`: it walks the slice
/// applying a predicate closure to each element and yields `Option<usize>`,
/// which `ok_or` converts into a `Result` in the same way as in `complete`.
/// `Vec::remove` then deletes at that index and shifts everything after it
/// left, so order survives — an O(n) cost that is irrelevant at this scale.
pub fn delete(todos: &mut Vec<Todo>, id: u32) -> Result<(), AppError> {
    let position = todos
        .iter()
        .position(|todo| todo.id == id)
        .ok_or(AppError::NotFound(id))?;
    todos.remove(position);
    Ok(())
}

/// Returns a reference to every todo whose text contains `query`.
///
/// Unlike the mutation helpers this hands back *borrowed* data: each element
/// of the result is a pointer into `todos` itself rather than a copy of a
/// `Todo`, so selecting a subset costs nothing beyond the vector that holds
/// the pointers. Both sides are lower-cased before comparing, which makes
/// the search case-insensitive at the price of one temporary `String` per
/// comparison — a fair trade for a store this size.
///
/// The explicit `<'a>` is required, and the reason is worth working through
/// because it is the one place in this crate where elision runs out. Rust
/// fills in omitted lifetimes using four rules, applied in order:
///
/// 1. every reference parameter gets its own anonymous lifetime;
/// 2. if there is exactly *one* input lifetime, it is assigned to every
///    output reference;
/// 3. otherwise, if the method takes `&self` or `&mut self`, that receiver's
///    lifetime is assigned to every output reference;
/// 4. otherwise the signature is an error and the lifetimes must be written.
///
/// Rule 1 gives `todos` and `query` two distinct lifetimes, so rule 2 has
/// nothing to pick (it needs exactly one), rule 3 does not apply (this is a
/// free function with no receiver), and rule 4 therefore demands the
/// annotation seen here.
///
/// Writing `todos: &'a [Todo]` and `Vec<&'a Todo>` ties the answer to the
/// store: the result may not outlive `todos`, so the compiler rejects any
/// caller that drops the vector while still holding the matches. `query`
/// keeps its own elided lifetime on purpose — it is only read during the
/// call, so it is allowed to be a shorter-lived temporary.
pub fn matching<'a>(todos: &'a [Todo], query: &str) -> Vec<&'a Todo> {
    // `needle` is lower-cased once, outside the closure, so the work is not
    // repeated for every todo the filter visits.
    let needle = query.to_lowercase();
    todos
        .iter()
        // The predicate receives `&&Todo` and auto-derefs twice to reach the
        // text; `filter` leaves the iterator's own `&Todo` items untouched.
        .filter(|todo| todo.text.to_lowercase().contains(&needle))
        // `collect` turns the chain back into a vector. Its target type is
        // known from this function's return type, so no turbofish is needed.
        .collect()
}

#[cfg(test)]
mod tests;
