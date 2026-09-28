# Instructions

How to build, run, and develop the this `project`.

## Prerequisites

A Rust toolchain with Cargo on your `PATH`:

```sh
rustup --version   # or: rustc --version && cargo --version
```

Rust 2021+ works; this crate uses `edition = "2024"`, so use Rust 1.85 or newer.

## Run it

Interactive session (no arguments — this is the normal way to use it):

```sh
cargo run
```

One-shot command (an argument is given, so it runs once and exits):

```sh
cargo run -- list
cargo run -- add buy milk
cargo run -- done 1
```

The `--` tells Cargo to stop passing options to itself and forward everything
after it to your program.

To use a real binary instead of `cargo run`, install it once:

```sh
cargo build --release
./target/release/todo          # or copy it somewhere on your PATH
```

## Commands

| Command        | Effect                                  |
| -------------- | --------------------------------------- |
| `add <text>`   | Create a todo; the text may span words  |
| `search <text>`| Show todos whose text contains `<text>` |
| `list`         | Show every todo and the done/total tally |
| `done <id>`    | Mark the todo with that id as completed |
| `delete <id>`  | Remove the todo with that id            |
| `quit` / `exit`| Leave the interactive session           |

Examples inside the `> ` prompt:

```text
> add read the book
added todo 1
> list
[ ]   1. read the book

0/1 done
> done 1
marked todo 1 as done
> quit
bye
```

Unknown commands, missing operands and bad ids print an error and the usage
line, then the prompt returns; in one-shot mode the process exits with
status `1`.


## Development

Every change must pass all three checks:

```sh
cargo fmt --check && cargo clippy -- -D warnings && cargo test
```

- `cargo fmt --check` — formatting.
- `cargo clippy -- -D warnings` — lints promoted to errors.
- `cargo test` — unit tests for argument parsing, store operations, storage
  round-trips (`persist`), listing output (`view`) and command execution
  (`tests`).

Useful while iterating: `cargo test cli` runs only the parser tests,
`cargo test persist` only the storage tests and `cargo test view` only the
listing tests, while `cargo watch -x test` re-runs everything on save if you
have `cargo-watch` installed.


Coding rules for this repository live in `AGENTS.md`.

---

