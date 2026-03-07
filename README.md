# Checkpoint.ts

[![codecov](https://codecov.io/gh/ErenayDev/checkpoint-ts/branch/main/graph/badge.svg)](https://codecov.io/gh/ErenayDev/checkpoint-ts)
[![CI](https://github.com/ErenayDev/checkpoint-ts/workflows/CI/badge.svg)](https://github.com/ErenayDev/checkpoint-ts/actions)
[![Crates.io](https://img.shields.io/crates/v/checkpoint-ts)](https://crates.io/crates/checkpoint-ts)
[![License: GPL-3.0-or-later](https://img.shields.io/crates/l/checkpoint-ts)](https://www.gnu.org/licenses/gpl-3.0.html)
[![Rust Version](https://img.shields.io/crates/msrv/checkpoint-ts?label=rust&color=orange)](https://www.rust-lang.org)
[![Downloads](https://img.shields.io/crates/d/checkpoint-ts.svg?color=blue)](https://crates.io/crates/checkpoint-ts)

Interactive checkpoint system for TypeScript/JavaScript.

## Quick Start

See [INSTALLATION.md](./doc/INSTALLATION.md)

## Key Features

When you run program, first parses your code with AST's and injects checkpointing functions.
Then injects runtime codes and then a TUI appears to you view.
When you wanna checkpoint in a point, the function name and its parameters, variables written to cache(in your .checkpoint folder)
If you wanna run the checkpointed function, just do it. You can edit the parameters, variables for the function.
You can profile execution times for each function.
Also you can edit whatever you want. really. try it yourself.

## Usage

### Basic Usage

```bash
checkpoint script.ts
```

### Runtime Selection

Currently only [Bun](https://bun.sh) is available. Check again in future for more runtimes

```bash
checkpoint script.ts
```

### Pre-instrumented Files (not available yet)

```bash
checkpoint --instrumented script.ts
```

## Supported Features

Covers like 85% TypeScript/JavaScript ecosystem codes. I'm planning optimize code and performance with switching to [oxc](https://oxc.rs) from [swc](https://swc.rs).
And for dead-code elimination, I wanna use the [jsshaker](https://github.com/kermanx/jsshaker) for O(n) to O(1) optimization

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
