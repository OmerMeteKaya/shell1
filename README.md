# Zesh

*(working name — will be renamed)*

A modern, POSIX-compliant shell written in Rust, focused on strict correctness and a fast, pleasant interactive experience.

> **Status: active development, pre-v1.** Core shell (lexer, parser, expansion, executor, builtins) is stable and extensively tested. Interactive UX features are functional and improving. See [Roadmap](#roadmap) for what's next.

## Features (pre-release)

- **Interactive UX**
  - Syntax highlighting
  - Ghost-text (inline) command completion and suggestions (e.g. typing `ca` suggests `cat`; typing `cat` suggests files in the current directory)
  - Smart `cd`
  - Visual history — pressing up shows the last 10 entries at once, so you can see exactly how far back to go
  - Visual search, same principle as history

## Correctness & Testing

- Full test suites of **git**, **busybox**, **toybox**, and **yash** pass against Zesh
- Builds and runs cleanly under **Arch Linux `makepkg`**
- Verified against the full **GNU toolchain**: `grep`, `sed`, `tar`, `make`, `automake`, `autoconf`, and `hello`, all passing
- Lexer and parser were fuzzed with **AFL++** for one continuous week: zero crashes, zero hangs, zero timeouts
- Currently working through remaining edge cases in **Gentoo ebuilds**

## Roadmap

- [ ] Strict error handling with readable try/catch-style blocks
- [ ] Type-safe scripting
- [ ] Multithreading
- [ ] Async scripting
- [ ] Plugin system

## Getting Started

```bash
cargo build --release
./target/release/zesh
```

## License

See [LICENSE](LICENSE).
