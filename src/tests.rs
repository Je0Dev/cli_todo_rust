use super::*;

/// Walks a todo through its whole life — created, listed, completed — using
/// the in-memory backend, then asks the backend itself what it now holds so
/// the assertion is about persisted state rather than about report wording.
#[test]
fn execute_add_list_done_round_trips_over_memory() {
    let mut storage = persist::Memory::new();
    let added = execute(&mut storage, Command::Add("read the book".into())).unwrap();
    assert_eq!(added, "added todo 1");
    let listing = execute(&mut storage, Command::List).unwrap();
    assert!(listing.contains("[ ]   1. read the book"));
    assert!(listing.contains("0/1 done"));
    execute(&mut storage, Command::Done(1)).unwrap();
    let todos = storage.load().unwrap();
    assert!(todos[0].done);
}

/// A bad id must surface as `AppError::NotFound` *and* stop the command
/// before the save on the last line of `execute`, so the store is left
/// exactly as it was — the `?` short-circuit, observed from outside.
#[test]
fn execute_reports_unknown_ids_and_persists_nothing() {
    let mut storage = persist::Memory::new();
    let error = execute(&mut storage, Command::Done(7)).unwrap_err();
    assert!(matches!(error, AppError::NotFound(7)));
    assert!(storage.load().unwrap().is_empty());
}

/// `execute` is generic over the backend, so it must also accept one whose
/// concrete type has been erased behind `dyn Persistence`: the extra `&mut`
/// is what lets the trait object satisfy the bound, because the blanket
/// impl makes `&mut dyn Persistence` itself a `Persistence`.
#[test]
fn execute_accepts_a_type_erased_storage() {
    let mut memory = persist::Memory::new();
    let mut erased: &mut dyn Persistence = &mut memory;
    let added = execute(&mut erased, Command::Add("buy milk".into())).unwrap();
    assert_eq!(added, "added todo 1");
    assert_eq!(erased.load().unwrap().len(), 1);
}

/// Search reaches the same store the listing reads, but returns only the
/// hits: the query is matched case-insensitively and the report counts them
/// as matches rather than as progress.
#[test]
fn execute_search_reports_only_the_matching_todos() {
    let mut storage = persist::Memory::new();
    execute(&mut storage, Command::Add("buy milk".into())).unwrap();
    execute(&mut storage, Command::Add("buy bread".into())).unwrap();
    let report = execute(&mut storage, Command::Search("MILK".into())).unwrap();
    assert!(report.contains("buy milk"));
    assert!(!report.contains("buy bread"));
    assert!(report.ends_with("\n1 match"));
}

/// A query that finds nothing is a normal outcome, so `execute` reports the
/// view's own explanation and still returns `Ok`.
#[test]
fn execute_search_reports_a_query_that_found_nothing() {
    let mut storage = persist::Memory::new();
    execute(&mut storage, Command::Add("buy milk".into())).unwrap();
    let report = execute(&mut storage, Command::Search("cheese".into())).unwrap();
    assert_eq!(report, "no todos matched that search");
}
