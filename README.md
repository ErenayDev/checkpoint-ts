# checkpoint.ts

> Stop rerunning 10 functions to debug the 11th.

Interactive checkpoint system for TypeScript/JavaScript development.
Inspect state, skip functions, inject values, profile execution.

**Like Minecraft parkour map checkpoints, but for your code**

Written in Rust 🦀 | Works with Bun, Node, Deno

---

## Quick Start

```bash
# Install
cargo install checkpoint-ts

# Run with checkpoints
checkpoint myfile.ts

# Use specific runtime
checkpoint myfile.ts --interpreter bun # deno, node
```

---

## How it works

Checkpoint transforms your code by injecting instrumentation points at function calls and variable assignments. When execution hits a checkpoint, it pauses and opens an interactive TUI where you can:

- Inspect current variable states
- Edit variable values on the fly
- Skip function execution and inject cached results
- Continue execution with modified state
- Profile execution time for each function

The system uses AST analysis to identify checkpoint opportunities and maintains a serializable execution context throughout your program's runtime.

---

## Features

**Interactive Debugging**
Pause execution at any function call and inspect the complete variable state without restarting your program.

**Function Skipping**
Skip expensive function calls during debugging and use cached or manually injected return values.

**State Manipulation**
Edit variables in real-time through the TUI and continue execution with your modifications.

**Execution Profiling**
Measure and track execution time for each function call with millisecond precision.

**Runtime Agnostic**
Works seamlessly with Bun, Node.js, and Deno through adaptive runtime detection.

---

## Installation

### From Cargo
```bash
cargo install checkpoint-ts
```

### From Source
```bash
git clone https://github.com/ErenayDev/checkpoint-ts
cd checkpoint.ts
cargo build --release
```

---

## Usage

### Basic Usage
```bash
checkpoint script.ts
```

### Runtime Selection
```bash
checkpoint script.ts --interpreter bun
checkpoint script.ts --interpreter node
checkpoint script.ts --interpreter deno
```

### Pre-instrumented Files
```bash
checkpoint --instrumented script.instrumented.ts
```

---

## TUI Interface

The terminal interface provides three main views:

**Checkpoint Navigator**
Shows current execution position, available checkpoints, and execution timeline.

**State Editor**
Interactive table for viewing and editing variable values at the current checkpoint.

**Execution Visualizer**
Performance metrics and execution flow visualization with timing information.

---

## Supported Features

- Function-level checkpoints
- Variable state inspection and editing
- Execution timing and profiling
- Cached result injection
- TypeScript and JavaScript support
- ES modules and CommonJS compatibility
- Async/await support (experimental)

---

## Limitations

- Circular references in objects are not fully supported
- Some complex TypeScript syntax may not be instrumented correctly
- Async operations have limited checkpoint support
- Maximum object depth for serialization is 10 levels

---

## Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

---

## License

Copyright (c) ErenayDev <erenaydev@proton.me>

This project is licensed under the MIT license ([LICENSE] or <http://opensource.org/licenses/MIT>)

[LICENSE]: ./LICENSE

---

## Sponsors

Special thanks to our sponsors who make this project possible:

**#1 Sponsors**
- [Become a sponsor](https://github.com/sponsors/ErenayDev)

**#2 Sponsors**
- [Become a sponsor](https://github.com/sponsors/ErenayDev)

**#3 Sponsors**
- [Become a sponsor](https://github.com/sponsors/ErenayDev)

---

## Acknowledgments

Built with:
- [SWC](https://swc.rs/) for TypeScript/JavaScript parsing
- [Ratatui](https://ratatui.rs/) for terminal user interface
- [Tokio](https://tokio.rs/) for async runtime support
