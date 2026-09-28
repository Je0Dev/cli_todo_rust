use super::*;

/// Builds a todo with an explicit state so each test can decide for itself
/// whether the fixture should read as completed.
fn todo(id: u32, text: &str, done: bool) -> Todo {
    Todo {
        id,
        text: text.into(),
        done,
    }
}

/// Borrows a store of fixtures as the vector of references a query view
/// expects — the same pointers `store::matching` would have produced.
fn matched(todos: &[Todo]) -> Vec<&Todo> {
    todos.iter().collect()
}

/// A store with nothing in it must teach the user something rather than
/// print a blank line, and that message belongs to the `all` constructor.
#[test]
fn an_empty_store_explains_how_to_add_one() {
    let listing = View::all(&[]).to_string();
    assert_eq!(listing, "no todos yet; add one with: todo add <text>");
}

/// The rendering itself: the mark, the right-aligned id, the text and the
/// tally all have to survive the move from `store::render` into `Display`.
#[test]
fn a_listing_marks_done_todos_and_counts_them() {
    let todos = vec![todo(1, "one", true), todo(2, "two", false)];
    let listing = View::all(&todos).to_string();
    assert!(listing.contains("[x]   1. one"));
    assert!(listing.contains("[ ]   2. two"));
    assert!(listing.ends_with("1/2 done"));
}

/// Query results summarise hits, not progress — and one hit is singular.
#[test]
fn a_query_view_counts_matches_in_the_plural_that_fits() {
    let todos = vec![todo(1, "buy milk", false)];
    let one = View::matched(matched(&todos)).to_string();
    assert!(one.ends_with("\n1 match"));
    let todos = vec![todo(1, "buy milk", false), todo(2, "buy bread", true)];
    let two = View::matched(matched(&todos)).to_string();
    assert!(two.ends_with("\n2 matches"));
}

/// A query that selects nothing gets its own explanation, not the one that
/// tells the user their store is empty.
#[test]
fn an_empty_query_view_says_nothing_matched() {
    let nothing = View::matched(Vec::new()).to_string();
    assert_eq!(nothing, "no todos matched that search");
}
