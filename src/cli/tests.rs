use super::*;

/// Turns word literals back into the owned arguments `parse` expects, so
/// each test can spell its input the way a shell would deliver it.
fn args(input: &[&str]) -> Vec<String> {
    input.iter().map(|word| (*word).to_string()).collect()
}

/// Narrows the result to "this failed as a usage error", which is what
/// every malformed-input test cares about; the payload carries the wording.
fn is_usage(result: Result<Command, AppError>) -> bool {
    matches!(result, Err(AppError::Usage(_)))
}

/// `add` takes the rest of the line verbatim, so the words must be rejoined
/// with single spaces rather than kept as separate operands.
#[test]
fn add_joins_every_remaining_word_into_one_text() {
    let parsed = parse(args(&["add", "buy", "milk"]));
    assert_eq!(parsed.unwrap(), Command::Add("buy milk".into()));
}

/// Blank text would create a todo nobody can read, so it is refused here,
/// before it can reach the store.
#[test]
fn add_rejects_blank_text() {
    assert!(is_usage(parse(args(&["add", "   "]))));
}

/// `search` shares `add`'s free-text handling, including the rejoining.
#[test]
fn search_joins_the_words_into_one_query() {
    let parsed = parse(args(&["search", "buy", "milk"]));
    assert_eq!(parsed.unwrap(), Command::Search("buy milk".into()));
}

/// A query with nothing to match would select every todo, which reads as a
/// bug rather than as an empty result, so it never becomes a `Search`.
#[test]
fn search_rejects_blank_text() {
    assert!(is_usage(parse(args(&["search", "   "]))));
}

/// The operand is parsed here rather than being coerced later, so a valid
/// id reaches the store as a number.
#[test]
fn done_parses_a_numeric_id() {
    assert_eq!(parse(args(&["done", "3"])).unwrap(), Command::Done(3));
}

/// Non-numeric text, negatives and surplus operands are all rejected, which
/// keeps bad ids from reaching the store in the first place.
#[test]
fn done_rejects_non_numeric_and_surplus_ids() {
    assert!(is_usage(parse(args(&["done", "x"]))));
    assert!(is_usage(parse(args(&["done", "1", "2"]))));
}

/// Both spellings of the farewell parse to the same variant, so the rest of
/// the program never has to compare strings.
#[test]
fn quit_and_exit_both_parse_to_quit() {
    assert_eq!(parse(args(&["quit"])).unwrap(), Command::Quit);
    assert_eq!(parse(args(&["exit"])).unwrap(), Command::Quit);
}

/// An unrecognised verb and a missing one alike produce a usage error, and
/// neither reaches `execute`.
#[test]
fn unknown_and_missing_commands_report_usage() {
    assert!(is_usage(parse(args(&["nope"]))));
    assert!(is_usage(parse(vec![])));
}
