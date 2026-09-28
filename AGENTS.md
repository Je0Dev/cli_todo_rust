# Agent Specifications

Binding rules for every file in this repository.

## File constraints

- **100 lines maximum per file, counting only the lines that are not comments.** `//`
  and `///` lines and the blank lines separating them are free, so a heavily documented
  file may well exceed 100 physical lines. Do not re-check a file's line count after
  comments have been added. Split into a new module when the code itself approaches the
  limit.
- **No TODO comments.** Unfinished work is tracked in the issue tracker, never inline.

## Commenting

- Use doc comments (`///`) in the professional, explanatory style: state what the item does,
  why it exists, and any contract the caller must respect (panics, errors, side effects).
- Document every public item. Private helpers need a comment only when intent is non-obvious.
- Explain the language machinery as well as the rationale. When a construct is subtle —
  ownership and borrowing, `?`, pattern matching, iterator chains, trait objects, format
  strings — spell out what the syntax means, how it behaves, and why it is written that way.
  Attach each explanation to the construct it describes; append paragraphs rather than
  replacing what is already there.

## Code style

- **Never repeat yourself.** Extract shared logic into one function, one constant, one match arm.
  Duplication is a defect, not a shortcut.
- **Prefer monolithic functions.** Build a handful of cohesive, self-contained functions that
  each own a complete step, rather than shattering logic into micro-helpers. A function should
  read as one continuous thought.

## Verification

Every change must pass before it is considered complete:

```
cargo fmt --check && cargo clippy -- -D warnings && cargo test
```
