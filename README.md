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
cargo run -- fmt --check

# Read from stdin and write to stdout
cat examples/quests.qk | cargo run -- fmt --stdin

# Format specific paths
cargo run -- fmt examples/quests.qk path/to/dir
```

The formatter preserves all comments and whitespace intent while normalizing indentation and spacing. It is idempotent.

### VS Code extension (local install)

You can build and install the Questicle VS Code extension locally:

```
make install
```

This will:
- build the Rust binaries (`qk`, `qk-lsp`)
- build and package the VS Code extension
- install the packaged `.vsix` into your VS Code

Notes:
- The extension looks for `qk-lsp` in `target/debug/` by default, or uses the `questicle.serverPath` setting if provided.
- The extension registers a formatter; use “Format Document” to format `.qk` files. It shells out to `qk fmt --stdin`.
- Packaged VSIX files are not committed to git (ignored by `.gitignore`).

## Language sketch

- Values: number, string, bool, null, list, map, function
- Variables: `let x = 1;`
- Functions: `fn add(a, b) { return a + b; }`
- Control flow: `if`, `while`, `for in`, `break`, `continue`
- Closures and lexical scoping
- Builtins: print, random, clock, len, keys, push, pop
- Events: `on("event", fn(e){ ... })` and `emit("event", data)`
- Host bridge: `host("spawn", { type: "npc", name: "Bob" })`

## Embedding

Link the `questicle` crate and implement the `Host` trait to integrate with your engine. See `src/host.rs`.
