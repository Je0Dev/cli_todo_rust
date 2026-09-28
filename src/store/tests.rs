use super::*;

/// Ids must stay unique among live todos even after a deletion recycles a
/// number, so this checks that `add` looks at the largest surviving id
/// rather than at the length of the vector.
#[test]
fn new_ids_do_not_collide_with_live_todos() {
    let mut todos = Vec::new();
    assert_eq!(add(&mut todos, "first".into()), 1);
    assert_eq!(add(&mut todos, "second".into()), 2);
    delete(&mut todos, 2).unwrap();
    assert_eq!(add(&mut todos, "third".into()), 2);
    let ids: Vec<u32> = todos.iter().map(|todo| todo.id).collect();
    assert_eq!(ids.len(), 2);
    assert_ne!(ids[0], ids[1]);
}

/// Both mutations must refuse an id the store does not carry, and the
/// `matches!` pattern proves the error carries the offending id back out.
#[test]
fn unknown_ids_are_reported_by_both_mutations() {
    let mut todos = Vec::new();
    add(&mut todos, "only".into());
    assert!(matches!(
        complete(&mut todos, 9),
        Err(AppError::NotFound(9))
    ));
    assert!(matches!(delete(&mut todos, 9), Err(AppError::NotFound(9))));
}

/// The selected todos are the caller's own elements, not copies of them:
/// `std::ptr::eq` compares addresses, so it only passes if the result
/// points back into the vector the query was handed.
#[test]
fn matching_returns_references_into_the_same_store() {
    let todos = vec![
        Todo {
            id: 1,
            text: "buy milk".into(),
            done: false,
        },
        Todo {
            id: 2,
            text: "buy bread".into(),
            done: false,
        },
    ];
    let hits = matching(&todos, "milk");
    assert_eq!(hits.len(), 1);
    assert!(std::ptr::eq(hits[0], &todos[0]));
}

/// The query and the text are both lower-cased before comparing, so the
/// search is case-insensitive in both directions.
#[test]
fn matching_ignores_case_on_either_side() {
    let todos = vec![Todo {
        id: 1,
        text: "Buy MILK".into(),
        done: false,
    }];
    assert_eq!(matching(&todos, "milk").len(), 1);
    assert_eq!(matching(&todos, "MILK").len(), 1);
}

/// A query that nothing satisfies is a normal answer, not an error, so the
/// caller receives an empty vector and decides how to report it.
#[test]
fn matching_reports_no_hits_as_an_empty_vector() {
    let mut todos = Vec::new();
    add(&mut todos, "buy milk".into());
    assert!(matching(&todos, "cheese").is_empty());
}
