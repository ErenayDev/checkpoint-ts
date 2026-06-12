# Checkpoint.ts

[![codecov](https://codecov.io/gh/ErenayDev/checkpoint-ts/branch/main/graph/badge.svg)](https://codecov.io/gh/ErenayDev/checkpoint-ts)
[![CI](https://github.com/ErenayDev/checkpoint-ts/workflows/CI/badge.svg)](https://github.com/ErenayDev/checkpoint-ts/actions)
[![Crates.io](https://img.shields.io/crates/v/checkpoint-ts)](https://crates.io/crates/checkpoint-ts)
[![License: GPL-3.0-or-later](https://img.shields.io/crates/l/checkpoint-ts)](https://www.gnu.org/licenses/gpl-3.0.html)
[![Rust Version](https://img.shields.io/crates/msrv/checkpoint-ts?label=rust&color=orange)](https://www.rust-lang.org)
[![Downloads](https://img.shields.io/crates/d/checkpoint-ts.svg?color=blue)](https://crates.io/crates/checkpoint-ts)

Interactive checkpoint system for TypeScript/JavaScript.

## Quick Start

See [INSTALLATION.md](./doc/src/INSTALLATION.md)

## Key Features

When you run the program, it first parses your code using ASTs and injects checkpointing functions.
Then it injects runtime code, and a TUI appears for you to interact with.
When you set a checkpoint, the function name along with its parameters and variables are written to the cache (in your `.checkpoint` folder).
To run the checkpointed function, simply do so—you can edit the parameters and variables beforehand.
You can also profile execution times for each function.
Edit anything you want—try it yourself!

## Usage

### Basic Usage

Currently, only [Bun](https://bun.sh) is available. Check again in future for more runtimes.

```bash
checkpoint -i script.ts
```

## Supported Features

Covers approximately 85% of TypeScript/JavaScript ecosystem code. See below.

## Known Limitations

### Concurrent execution and call stack

The runtime tracks call stack depth and caller information using a single
shared array. This works correctly for sequential code (the most common case),
but produces inaccurate stack information when multiple async operations are
executed concurrently — for example, `await Promise.all([fetchA(), fetchB()])`.

In such cases, the displayed stack depth and caller name reflect the order in
which checkpoints were resumed by the user, not the actual call hierarchy. The
execution itself is unaffected; only the visual stack metadata can be misleading.

A future release will track call stack per async context using `AsyncLocalStorage`
or equivalent mechanisms.

### Container function timing

Functions that contain nested checkpoints (e.g. `main`) report a duration that
includes the time spent waiting for user input on inner checkpoints. The
profiler currently shows inclusive (wall-clock) time per function. Exclusive
("self") time will be calculated using stack depth in a future release.

### Method chains and curry patterns

The transformer skips checkpoint instrumentation for:

- Method chains: `foo().bar()`, `new Foo().bar()`
- Curry/factory calls: `chalk.hex("#color")("text")`
- IIFE patterns: `(() => fn)()`

This is a deliberate trade-off to avoid duplicate evaluation of side-effecting
expressions (e.g., constructors). Top-level calls within these chains are still
instrumented when possible.

## Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

## License

Copyright (c) ErenayDev <erenaydev@proton.me>

This project is licensed under the GPL-3.0-or-later license ([LICENSE] or <https://www.gnu.org/licenses/gpl-3.0.html>)

[LICENSE]: ./LICENSE

## Sponsors

Special thanks to our sponsors who make this project possible:

<p align="center">
  <a href="https://github.com/sponsors/ErenayDev">
    <img src="https://raw.githubusercontent.com/ErenayDev/ErenayDev/refs/heads/main/sponsorkit/sponsors.svg" alt="ErenayDev's sponsors" />
  </a>
</p>

## Acknowledgments

Built with:

- [SWC](https://swc.rs/) for TypeScript/JavaScript parsing
- [Ratatui](https://ratatui.rs/) for terminal user interface
- [Tokio](https://tokio.rs/) for async runtime support
