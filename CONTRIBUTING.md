# Contributing to Farx

Thank you for your interest in contributing! Farx is open source and welcomes contributions of all kinds — bug fixes, new features, themes, documentation, and more.

## Getting Started

### Prerequisites

- [Rust](https://rustup.rs/) (stable toolchain)
- Git

### Setup

```sh
git clone https://github.com/ashishtyagi10/farx.git
cd farx
cargo build
cargo run
```

A pre-commit hook runs `cargo fmt --check` and `cargo check` automatically on every commit.

### Running

```sh
cargo run                    # Run in debug mode
cargo run --release          # Run in release mode
cargo run -- --keydebug      # Debug terminal key events
```

## Project Structure

Farx is a Cargo workspace with six crates:

```
farx/
├── crates/
│   ├── farx-app/       # Binary entry point, CLI, auto-update
│   ├── farx-core/      # Config, keymaps, actions, types (no UI deps)
│   ├── farx-fs/        # Filesystem operations (copy, move, delete)
│   ├── farx-ui/        # TUI components, themes, event loop
│   ├── farx-ai/        # AI agent (multi-provider LLM integration)
│   └── farx-plugin/    # Plugin system (WIP)
├── config/             # Default configuration files
└── docs/               # GitHub Pages site
```

### Dependency Graph

```
farx-app (binary)
├── farx-core
├── farx-fs
└── farx-ui
    ├── farx-core
    ├── farx-fs
    └── farx-ai
        └── farx-core

farx-plugin
└── farx-core
```

**Rule:** `farx-core` has zero internal dependencies. All other crates depend on it. `farx-ui` is the only crate that depends on `farx-ai` and `farx-fs`.

## Architecture

### Event Flow

A keypress flows through these layers:

```
Terminal (crossterm)
  → EventHandler (farx-ui/src/event.rs)     # Async bridge to tokio channel
    → App::handle_key_event (app.rs)         # Routes to active modal or keymap
      → KeyMap::resolve_panel (keymap.rs)    # Maps key combo → Action enum
        → App::dispatch(action) (app.rs)     # Mutates state based on Action
          → App::render(frame) (app.rs)      # Draws the new state
```

### Key Types

| Type | Location | Purpose |
|------|----------|---------|
| `Action` | `farx-core/src/action.rs` | All possible user actions (CursorUp, CopyDialog, Quit, etc.) |
| `KeyMap` | `farx-core/src/keymap.rs` | Maps key combos to Actions |
| `AppConfig` | `farx-core/src/config.rs` | All configuration (general, UI, panels, AI) |
| `TreeState` | `farx-core/src/tree.rs` | Hierarchical directory tree state |
| `PanelState` | `farx-core/src/types.rs` | Flat file list panel state |
| `FileEntry` | `farx-core/src/types.rs` | Single file/directory metadata |
| `Theme` | `farx-ui/src/theme.rs` | Color scheme for all UI elements |
| `AiAgent` | `farx-ai/src/agent.rs` | Multi-provider LLM client |
| `App` | `farx-ui/src/app.rs` | Top-level application state and logic |

### Modal Priority

When handling key events, the deepest active modal consumes input first:

1. Editor (F4) — full screen
2. Viewer (F3) — full screen
3. Feedback (inline confirmations)
4. Help (F1)
5. Menu (F9)
6. Search (Alt+F7)
7. AI Bar (Ctrl+Space)
8. Dialog (input prompts)
9. Command line (if has input)
10. Panel keybindings (default)

## Common Contributions

### Adding a New Theme

Edit `crates/farx-ui/src/theme.rs`:

1. Add a constructor function:

```rust
pub fn my_theme() -> Self {
    let bg = Color::Rgb(30, 30, 40);
    let fg = Color::Rgb(200, 200, 210);
    // ... define all colors ...
    Self {
        name: "my-theme",
        panel_bg: bg,
        panel_fg: fg,
        // ... fill all fields (see existing themes for reference) ...
    }
}
```

2. Register it in `by_name()`:

```rust
"my-theme" => Self::my_theme(),
```

3. Add it to `available()`:

```rust
pub fn available() -> &'static [&'static str] {
    &["far-classic", "tokyo-night", "catppuccin", "dracula", "gruvbox", "my-theme"]
}
```

Every field in `Theme` must be set. Copy an existing theme as a starting point.

### Adding a Keybinding

Edit `crates/farx-core/src/keymap.rs`:

1. Add the `Action` variant in `crates/farx-core/src/action.rs` if it doesn't exist
2. Add the key mapping in `KeyMap::far_defaults()`
3. Handle the action in `App::dispatch()` in `crates/farx-ui/src/app.rs`

### Adding a File Operation

1. Implement the operation in `crates/farx-fs/src/ops.rs`
2. Add an `Action` variant in `farx-core`
3. Wire it up in `App::dispatch()`

### Adding a New UI Component

1. Create the file in `crates/farx-ui/src/components/`
2. Export it in `crates/farx-ui/src/components/mod.rs`
3. Add state to `App` struct in `app.rs`
4. Handle key events in `App::handle_key_event()`
5. Render in `App::render()`

### Adding an AI Provider

Edit `crates/farx-ai/src/agent.rs`:

1. Add a variant to `ApiProvider` enum
2. Add request/response structs
3. Handle the new provider in `query()` and `suggest()` methods
4. Map the provider string in `AiAgent::new()`

## Code Style

- Run `cargo fmt` before committing (the pre-commit hook enforces this)
- Keep `cargo clippy` warnings clean
- Prefer `anyhow::Result` for error handling in application code
- Use `thiserror` for library error types in `farx-core`
- Use `tracing` macros (`info!`, `warn!`, `error!`) instead of `println!` / `eprintln!`

## Coverage

Install and run coverage locally:

```sh
cargo install cargo-llvm-cov
cargo llvm-cov clean --workspace
cargo llvm-cov --workspace --all-features --html
cargo llvm-cov --workspace --all-features --lcov --output-path lcov.info
```

## Pull Request Process

1. Fork the repository
2. Create a feature branch: `git checkout -b my-feature`
3. Make your changes
4. Ensure `cargo fmt --check` and `cargo check --workspace` pass
5. Commit with a clear message
6. Push and open a Pull Request against `main`

Keep PRs focused — one feature or fix per PR. If a change is large, open an issue first to discuss the approach.

## Reporting Bugs

Open an issue at [github.com/ashishtyagi10/farx/issues](https://github.com/ashishtyagi10/farx/issues) with:

- Your OS and terminal emulator
- Steps to reproduce
- Expected vs actual behavior
- Terminal output or screenshots if relevant

## License

By contributing, you agree that your contributions will be licensed under the same dual license as the project: MIT or Apache-2.0, at your option.
