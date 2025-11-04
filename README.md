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

## License

MIT or Apache-2.0
