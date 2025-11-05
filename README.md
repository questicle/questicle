# Questicle

Questicle is a general purpose scripting language for games. Use it to script characters, events, NPC behavior, dialogue, fights, actions, and more — anything that's not the raw game engine code.

Status: early prototype.

## Quick start

- Build

```
cargo build
```

- REPL

```
cargo run -- -r
```

- Run a file

```
cargo run -- examples/hello.qk
```

### Formatter

Format files with the CLI subcommand:

```
# Format all .qk files under current directory
cargo run -- fmt

# Check only (exit 1 if changes would be made)
# Questicle

Questicle is a general-purpose scripting language for games. Use it to script NPC behavior, dialogue, combat, quests, triggers, and more—everything you don’t want to hard-code into your engine.

Status: early prototype, but complete enough to write real scripts. Includes a REPL, a CLI, a language server, a VS Code extension, and a robust comment-preserving formatter.

## Install & build

Build the toolchain (CLI, LSP, library):

```
cargo build
```

Run the REPL:

```
cargo run -- -r
```

Run a script file:

```
cargo run -- examples/hello.qk
```

## Formatter (questicle fmt)

Questicle ships with a robust, comment-preserving formatter. It never drops or reorders comments and only normalizes whitespace and indentation.

Examples:

```
# Format all .qk files under the current dir
cargo run -- fmt

# Check only (exit 1 if changes would be made)
cargo run -- fmt --check

# Read from stdin, write to stdout
cat examples/quests.qk | cargo run -- fmt --stdin

# Format specific files/dirs
cargo run -- fmt examples/quests.qk examples/
```

Formatting rules:
- Indentation: 2 spaces
- One statement per line
- Space around binary operators (a + b)
- Commas and colons followed by a space
- Object literals: `{ key: value, key2: value2 }`; block forms align properties consistently
- Braces style: `fn name(params) {` (opening brace on same line), body indented, closing brace aligned
- Collapse 3+ blank lines to max 2
- Preserve end-of-line comments on their lines and keep all comments in original order
- Tolerant of incomplete code (falls back to token-based printing)

The formatter is idempotent: running it twice yields the same output.

## VS Code extension

You can build and install the Questicle VS Code extension locally:

```
make install
```

This will build the Rust binaries (`qk`, `qk-lsp`), package the extension, and install the VSIX into VS Code.

Notes:
- The extension looks for `qk-lsp` in `target/debug/` by default, or uses the `questicle.serverPath` setting if provided.
- The extension registers a document formatter; run “Format Document” on `.qk` files. It shells out to `qk fmt --stdin` under the hood.

## Language overview

- Values: `number`, `string`, `bool`, `null`, `list`, `map`, `function`
- Variables: `let x = 1;`
- Functions: `fn add(a, b) { return a + b; }`
- Control flow: `if`, `while`, `for in`, `break`, `continue`
- Closures and lexical scoping
- Builtins: `print`, `random`, `clock`, `len`, `keys`, `push`, `pop`, `on`, `emit`, `host`
- Events: `on("event", fn(e){ ... })` and `emit("event", data)`

## Examples

See the `examples/` folder for scripts like quests, NPC behavior, inventory, and more:

```
cargo run -- examples/quests.qk
```

## Embedding

Link the `questicle` crate and implement the `Host` trait to integrate with your engine. See `src/host.rs` for the default stub. The interpreter exposes `host(op, payload)` to call into your game host.

## Development

Run tests (includes formatter tests and runs every example script):

```
cargo test
```

Format all `.qk` files:

```
cargo run -- fmt
```

Release checklist:
- Ensure `cargo test` passes.
- Run `cargo run -- fmt --check` to confirm no formatting drift.
- Package/install the VS Code extension with `make install`.
