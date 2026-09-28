use std::fmt::{self, Display, Formatter};

use crate::store::Todo;

/// A borrowed snapshot of the todos that should be printed.
///
/// The application used to build its listing as an owned `String` inside the
/// store module; this type moves the same work behind a value that *borrows*
/// the todos instead, which is what makes the lifetime visible. The `'a` on
/// the struct is its single lifetime parameter: every `View<'a>` promises
/// that the todos it points at will live at least as long as the view
/// itself, so the compiler is free to check that promise once, here, rather
/// than at every use site. Nothing is copied — the `Vec<&'a Todo>` is a
/// vector of pointers into the caller's data.
///
/// The fields are private on purpose. A view is only ever built through
/// `all` or `matched`, so the pair of messages (what to say when the list is
/// empty, and how to summarise it) is fixed by the constructor and cannot
/// drift apart — the usual reason to hide fields is to keep an invariant
/// that the type name is supposed to guarantee.
pub struct View<'a> {
    /// The todos to print, borrowed from whichever store produced them.
    todos: Vec<&'a Todo>,
    /// Sentence to print in place of an empty list.
    ///
    /// The type is `&'static str`: a string literal, which lives in the
    /// binary's read-only data for the whole process. Because `'static`
    /// outlives everything, a view may carry one without owning a `String`
    /// or borrowing from anyone.
    empty: &'static str,
    /// Whether this view shows query results rather than the whole store,
    /// which decides both the summary line and the plural of "match".
    from_query: bool,
}

impl<'a> View<'a> {
    /// Views every todo in the store, reporting progress through it.
    ///
    /// `todos: &'a [Todo]` borrows the slice for as long as the view lives,
    /// and the returned `View<'a>` carries the same `'a` — the two are tied
    /// together, so a caller cannot outlive the vector it lent. Callers pass
    /// `&todos` for a `Vec<Todo>` without saying anything else: deref
    /// coercion widens `&Vec<Todo>` to the `&[Todo]` asked for here.
    ///
    /// `.iter().collect()` builds the `Vec<&'a Todo>` field: the iterator
    /// over a slice yields references with exactly the slice's lifetime, so
    /// the collected pointers inherit `'a` and the `collect` needs no
    /// annotation — the field's type is enough to choose the collection.
    pub fn all(todos: &'a [Todo]) -> Self {
        View {
            todos: todos.iter().collect(),
            empty: "no todos yet; add one with: todo add <text>",
            from_query: false,
        }
    }

    /// Views the matches a search produced, reporting how many there are.
    ///
    /// The vector arrives by value because it was built for this view alone:
    /// taking `Vec<&'a Todo>` moves the pointers out of the caller in one
    /// allocation instead of copying them, and the lifetime is unchanged by
    /// the move — moving *data* never extends how long it may be borrowed.
    pub fn matched(matches: Vec<&'a Todo>) -> Self {
        View {
            todos: matches,
            empty: "no todos matched that search",
            from_query: true,
        }
    }
}

impl Display for View<'_> {
    /// Writes the listing into `f`, the formatter handle the `to_string`
    /// machinery created for us.
    ///
    /// `impl Display for View<'_>` attaches the trait to the type with an
    /// *elided* lifetime: `'_` says "some lifetime, chosen by the caller",
    /// which is enough here because `fmt` never stores the handle — it only
    /// borrows `self` for the duration of this call. Once `Display` is
    /// implemented, every `format!`/`to_string` on a `View` comes for free.
    ///
    /// The return type is `fmt::Result`, not `String`: the method does not
    /// build a string, it *writes* into a destination, and the `?` after
    /// each macro propagates an `Err` (a full or closed buffer) straight out
    /// of `fmt`. `write!` and `writeln!` are macros rather than functions so
    /// they can forward the format string to the compiler; `writeln!` is
    /// `write!` plus a newline.
    ///
    /// The early `return f.write_str(self.empty);` exits before any of the
    /// iterator work below is reached: `write_str` is the allocation-free
    /// way to emit a string that is already complete, and it happens to be
    /// `fmt::Result` too, so it can be returned directly.
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if self.todos.is_empty() {
            return f.write_str(self.empty);
        }
        // `filter` keeps the completed todos and `count` consumes the
        // iterator into a plain `usize`; neither allocates.
        let done = self.todos.iter().filter(|todo| todo.done).count();
        let total = self.todos.len();
        // `&self.todos` reborrows the vector so the loop does not move it —
        // `fmt` takes `&self`, and everything here stays behind that shared
        // borrow. Each element is already a `&&Todo`, which auto-derefs.
        for todo in &self.todos {
            // An `if` is an expression, so it yields the character bound to
            // `mark`; Rust has no ternary operator.
            let mark = if todo.done { 'x' } else { ' ' };
            // `{:>3}` right-aligns the id in a three-character field while
            // `{mark}` and the other `{...}` captures read variables from
            // scope by name rather than by position.
            writeln!(f, "[{mark}] {:>3}. {}", todo.id, todo.text)?;
        }
        // The blank line and the summary mirror what the old `render`
        // produced: a `write!` (no newline) after the loop's final newline.
        if self.from_query {
            let noun = if total == 1 { "match" } else { "matches" };
            write!(f, "\n{total} {noun}")
        } else {
            write!(f, "\n{done}/{total} done")
        }
    }
}

#[cfg(test)]
mod tests;
