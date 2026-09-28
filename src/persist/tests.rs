use super::*;

/// Builds a path in the system temporary directory that no other test in
/// this process uses, so the file-based tests cannot collide with each other
/// or with the developer's real `todos.json`.
///
/// `std::process::id()` is the operating system's identifier for this test
/// run, which keeps parallel runs on the same machine apart as well, and
/// `tag` distinguishes the tests inside one run.
fn temp_path(tag: &str) -> PathBuf {
    std::env::temp_dir().join(format!("todo-{tag}-{}.json", std::process::id()))
}

/// A fresh in-memory backend starts empty, and whatever `save` writes comes
/// back unchanged through `load` — the round trip every other test relies on.
#[test]
fn memory_round_trips_saved_todos() {
    let mut storage = Memory::new();
    assert!(storage.load().unwrap().is_empty());
    storage
        .save(&[Todo {
            id: 1,
            text: "buy milk".into(),
            done: false,
        }])
        .unwrap();
    let loaded = storage.load().unwrap();
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].text, "buy milk");
}

/// The same round trip over the real backend, in a file of its own: bytes
/// are written, parsed back, and compared structurally with `assert_eq!`,
/// which is why `Todo` derives `PartialEq`. The file is removed at the end
/// so repeated runs do not accumulate debris in the temporary directory.
#[test]
fn json_file_round_trips_saved_todos() {
    let path = temp_path("round-trip");
    let todos = vec![Todo {
        id: 3,
        text: "read the book".into(),
        done: true,
    }];
    let mut storage = JsonFile::new(&path);
    storage.save(&todos).unwrap();
    assert_eq!(storage.load().unwrap(), todos);
    fs::remove_file(&path).unwrap();
}

/// A first run has no document at all, and that is not a failure: the
/// backend answers with an empty store so the application never has to
/// special-case a fresh checkout.
#[test]
fn a_missing_document_is_an_empty_store() {
    let path = temp_path("missing");
    let storage = JsonFile::new(&path);
    assert!(storage.load().unwrap().is_empty());
}

/// A document that exists but cannot be parsed must be reported, never
/// replaced: the second assertion re-reads the file and finds it untouched,
/// proving no code path "repaired" the user's data by overwriting it.
#[test]
fn an_unreadable_document_is_reported_and_left_alone() {
    let path = temp_path("corrupt");
    fs::write(&path, "not json at all").unwrap();
    let storage = JsonFile::new(&path);
    assert!(matches!(storage.load(), Err(AppError::Json(_))));
    assert_eq!(fs::read_to_string(&path).unwrap(), "not json at all");
    fs::remove_file(&path).unwrap();
}
