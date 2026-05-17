# Copilot Instructions for Farx

## Build, test, and lint commands

Use Cargo workspace commands from the repository root:

```sh
# build
cargo build
cargo build --release

# run app
cargo run
cargo run --release
cargo run -- --keydebug

# lint/format checks
cargo fmt --check
cargo clippy --workspace --all-targets -- -W clippy::all
cargo check --workspace

# tests
cargo test --workspace

# run a single integration test
cargo test -p farx-ui --test features_test test_sort_toggle_reverses_order
```

## High-level architecture

Farx is a Cargo workspace with six crates:

- `crates/farx-app`: binary entrypoint (`farx`), CLI flags, terminal setup/teardown, update flow handoff.
- `crates/farx-core`: shared domain types/config/action enums/keymap/layout/tree/tab abstractions.
- `crates/farx-fs`: filesystem operations (copy/move/delete/archive/duplicates/read directory).
- `crates/farx-ui`: main app state machine, event loop integration, rendering, command handling, components.
- `crates/farx-ai`: AI provider client abstraction used by UI.
- `crates/farx-plugin`: Lua plugin engine and plugin command execution.

Runtime flow (cross-file):

1. `farx-app/src/main.rs` initializes terminal and starts loop.
2. `farx-ui/src/event.rs` converts crossterm events into async app events (`Key`, `Mouse`, `Resize`, `Tick`).
3. `farx-ui/src/app.rs::handle_key_event` applies modal-priority input handling.
4. `farx-core/src/keymap.rs` resolves key combos to `Action`.
5. `farx-ui/src/app.rs::dispatch` executes `Action` and mutates app/tree state.
6. `farx-ui/src/app.rs::render` draws full-screen modals first, then panels/overlays, with update modal last.

## Key conventions in this codebase

- **Action wiring is multi-file and required:** new behavior usually spans `farx-core/src/action.rs` (new enum variant), `farx-core/src/keymap.rs` (key mapping and aliases), and `farx-ui/src/app.rs` (dispatch + rendering/input state). CONTRIBUTING explicitly follows this pattern.

- **Input priority is intentional:** `handle_key_event` processes full-screen modes and overlays before panel keybindings (editor/viewer/diff/embedded terminal/feedback/help/update/menu/search/etc). Preserve this ordering when adding new modal UI.

- **Tree state is the navigation source of truth:** `dispatch` routes cursor/navigation/selection through active `TabGroup`/`TreeState`; avoid implementing navigation directly against `PanelState` only.

- **Config-driven key remaps use string parsing:** `[keybindings]` in `config.toml` is parsed by `parse_key_combo` + `parse_action` in `keymap.rs`; action aliases are normalized by lowercasing and removing `-`/`_`.

- **Command line behavior is hybrid, not shell-only:** `smart_execute_command` in `app.rs` routes slash commands (`/...`) and `cd` as built-ins, then heuristically chooses shell execution vs AI query.

- **Release process is tag-driven GitHub Actions:** `.github/workflows/release.yml` builds multi-target artifacts on `v*` tags and publishes release assets/checksums. Prefer tagging (`git tag vX.Y.Z && git push origin vX.Y.Z`) to trigger release packaging.
