# Crew Chat Panes + Plugin System — Design Doc

**Date:** 2026-06-21
**Status:** Approved for planning
**Builds on:** Crew Plans 1–3 (GPU terminal → per-cell rendering → multi-pane agent grid)

---

## 1. Summary

Add **human team chat** (Slack / Teams / other tools) to Crew as a new **pane type**
in the existing agent grid, backed by a **subprocess plugin system**. A pane is now
either a **Terminal** (a shell/agent over a PTY — today's behavior) or a **Chat** (a
message view driven by an external plugin process). Both render *as cells*, so the
grid, focus, click, and `Cmd`-chord machinery work on chat panes unchanged, and
`crew-render` stays terminal-agnostic.

This is the **human** chat surface — distinct from the planned agent-orchestrator
box (Plan 4). Agents and people get separate surfaces; this spec covers people.

---

## 2. Decisions (locked during brainstorming)

| Decision | Choice |
|---|---|
| What flows through it | Human team comms (Slack/Teams/…), separate from the agent orchestrator |
| Where it lives | A **pane** in the agent grid (a tile, like a terminal) — no new layout region |
| Plugin model | **Subprocess + newline-delimited JSON protocol** over stdio (LSP/MCP-style) |
| Chat rendering | Reuse `CellView` — chat panes lay messages + input out **as cells** (monospace) |
| v1 scope | Foundation + a trivial **reference plugin**; real Slack/Teams are later plugins |

**Why a pane, not a dock:** Crew's hard rule is *one layout, no layout switching*.
A chat pane adds zero new regions — it reuses the grid. **Why cells, not a rich GPU
path:** keeps `crew-render` pure and the look consistent with terminals; the cost
(monospace, no avatars/embeds) is acceptable for a terminal-native chat and can be
revisited later.

---

## 3. Goals & non-goals

### Goals (v1)
- A `Chat` pane type that tiles alongside terminal panes in the grid.
- A `crew-plugin` host that spawns a provider subprocess and exchanges JSON lines.
- A documented, versioned **chat plugin protocol** (handshake, channels, message
  events, send).
- A `ChatPane` that renders a channel's messages + an input line as cells, and
  routes focused-pane keystrokes to its input (Enter → send).
- A **reference echo plugin** (a workspace binary) proving the pipe end-to-end.
- `Cmd+J` spawns a chat pane bound to the configured provider.

### Non-goals (v1)
- **Real Slack/Teams/Discord plugins** — they're separate programs speaking the
  protocol; built after the protocol is stable.
- OAuth flows, threads, reactions, attachments, presence, typing indicators.
- Rich rendering (avatars, images, embeds) — chat is monospace cells in v1.
- Plugin discovery/marketplace, hot-reload, sandboxing beyond process isolation.
- The agent orchestrator box (separate surface, Plan 4).

---

## 4. Architecture

```
┌──────────────────────── Crew (one window, the agent grid) ─────────────────────┐
│  pane 0 [Terminal]      pane 1 [Chat: echo]        pane 2 [Terminal]            │
│   PtyTerm → cells        ChatPane → cells           PtyTerm → cells              │
│                              │  ▲                                                │
│                       send  │  │ message events                                 │
│                              ▼  │                                                │
│                       ┌─ crew-plugin host ─┐                                     │
│                       │ spawns subprocess  │  stdin/stdout JSON lines            │
│                       └─────────┬──────────┘                                     │
│  Renderer.frame(&[PaneScene])   │   reader thread → channel (PtyTerm pattern)    │
└─────────────────────────────────┼───────────────────────────────────────────────┘
                                  ▼
                         echo-plugin (separate process)
```

**Key invariant preserved:** every pane — Terminal or Chat — produces
`Vec<CellView>` for its rect. `crew-render` renders cells and is unaware of chat.
`crew-app` owns the polymorphism.

### Module / crate boundaries
- **`crew-plugin`** *(new crate)* — the protocol types (`serde`-derived) + the host:
  spawn a provider via `std::process::Command` with piped stdio; a reader thread
  parses JSON lines from stdout into `PluginEvent`s forwarded over an `mpsc` channel
  (same pattern as `PtyTerm`'s reader); a writer sends `PluginCommand`s as JSON
  lines to stdin. Knows nothing about rendering. Unit-testable against a mock child
  (e.g. a `cat`-like echo or an in-process fake).
- **`crates/crew-echo-plugin`** *(new bin)* — reference plugin: reads JSON commands
  on stdin, writes JSON events on stdout. On `hello` → `ready` with one channel
  `general`; on `send` → emits a `message` echoing the text back (sender `echo`).
- **`crew-app`** — `PaneContent { Terminal(TermPane) | Chat(ChatPane) }`; `ChatPane`
  (plugin handle + channel + `Vec<Message>` + input buffer + cell layout);
  keyboard-routing branch on pane type; `Cmd+J` spawn; provider config.

---

## 5. The chat plugin protocol (v1)

Newline-delimited JSON. One JSON object per line, each with a `"type"` tag. Protocol
version negotiated in the handshake (`"v":1`).

**Crew → plugin (`PluginCommand`):**
```json
{"type":"hello","v":1}
{"type":"subscribe","channel":"general"}
{"type":"send","channel":"general","text":"hello team"}
```

**Plugin → Crew (`PluginEvent`):**
```json
{"type":"ready","v":1,"provider":"echo","channels":["general"]}
{"type":"message","channel":"general","sender":"bob","text":"hey","ts":"2026-06-21T10:00:00Z"}
{"type":"error","message":"auth failed"}
```

Rules: the plugin MUST emit `ready` before any `message`. Unknown line / parse error
→ the host logs and drops that line (never panics). Plugin exit / broken pipe → the
host emits a synthetic `error` event and marks the pane disconnected. `ts` is an
opaque string (display/order only in v1).

---

## 6. Components

### 6.1 `crew-plugin` host
- `PluginCommand` / `PluginEvent` enums (`#[serde(tag="type")]`).
- `struct Plugin { child, writer, rx: Receiver<PluginEvent> }` with
  `Plugin::spawn(cmd: &str, args: &[String]) -> Result<Plugin>`,
  `send(&self, PluginCommand)`, `try_recv(&self) -> Vec<PluginEvent>` (non-blocking
  drain). Reader thread: `BufReader::lines()` → parse → `tx.send`.
- Tested headless: spawn the echo plugin (or a fake), send `hello`, assert a `ready`
  event arrives; send `send`, assert the echoed `message`.

### 6.2 `crew-echo-plugin` (reference)
- A ~50-line Rust bin. Loop over stdin lines; match command; print events. No deps
  beyond `serde`/`serde_json` (shared with the host).

### 6.3 `ChatPane` (in `crew-app`)
- State: `plugin: Plugin`, `channel: String`, `messages: Vec<Message>`,
  `input: String`, `connected: bool`.
- `poll()` — drain `plugin.try_recv()`; append messages; flip `connected`/record
  errors. Called in `about_to_wait` alongside terminal `try_read` (any new event →
  redraw).
- `cells(&self, cols, rows) -> Vec<CellView>` — lay out the last N messages bottom-up
  (`sender:` in the neon accent fg, text in default fg, wrapped to `cols`), reserve
  the bottom row for `> {input}` with a cursor. This replaces the terminal `cells()`
  for chat panes when `build_scenes` assembles the frame.
- Input: when a Chat pane is focused and a non-`Cmd` key arrives, edit `input`
  (printable → push; Backspace → pop; Enter → `plugin.send(send{channel,input})`
  then clear). `Cmd`-chords still go to pane management.

### 6.4 `crew-app` integration
- `Pane.content: PaneContent`. `build_scenes` asks each content for its `cells`
  (Terminal → PtyTerm cells via `to_cellviews`; Chat → `ChatPane::cells`).
- `spawn_chat_pane()` — spawn the configured plugin via `crew-plugin`, build a
  `ChatPane`, push as a new grid pane, focus it, relayout. Bound to `Cmd+J`.
- Provider config v1: a constant/env (`CREW_CHAT_PLUGIN`) giving the plugin command;
  defaults to the built `crew-echo-plugin` binary path.

---

## 7. Data flow

**Inbound:** plugin stdout → reader thread → `PluginEvent` channel → `ChatPane.poll`
(in `about_to_wait`) → `messages` updated → redraw → `ChatPane.cells` → `PaneScene`
→ `Renderer.frame`.

**Outbound:** focused Chat pane + Enter → `plugin.send(send{...})` → JSON line to
plugin stdin → plugin posts to the service (echo: loops it straight back).

**Focus/keys:** unchanged routing — `Cmd`-chords manage panes; non-`Cmd` keys go to
the focused pane, which now branches: Terminal → PTY bytes; Chat → input edit.

---

## 8. Risks & mitigations

| Risk | Mitigation |
|---|---|
| Plugin subprocess hangs / floods | Reader thread + bounded drain per frame; never block the UI thread (PtyTerm pattern). |
| Malformed JSON from a plugin | Parse per line; log + drop bad lines; never panic. |
| Pane polymorphism bloats `app.rs` past 200 lines | `ChatPane` + layout live in a new `chat.rs`; `PaneContent` thin. Keep every `.rs` ≤200. |
| `crew-render` accidentally learns about chat | Hard line: chat → `CellView` in `crew-app`; renderer signature unchanged. |
| Keyboard routing regressions for terminals | Branch only on focused pane type; terminal path byte-identical to today. |
| Plugin auth/secrets (future Slack) | Out of v1; plugins own their own auth via their own env/config — Crew never handles tokens. |

---

## 9. v1 scope vs later

**v1 (this spec):** `crew-plugin` host + protocol; `crew-echo-plugin`; `PaneContent`
polymorphism; `ChatPane` (render + input); `Cmd+J` spawn; tests (host roundtrip via
echo plugin, chat-cell layout).

**Later:** real Slack/Teams/Discord plugins; threads/reactions/attachments; rich
rendering; multiple simultaneous providers; channel switcher UI; presence/typing.

---

## 10. Testing strategy

- **`crew-plugin`:** integration test spawning `crew-echo-plugin` — `hello`→`ready`,
  `send`→echoed `message`; malformed-line resilience (feed junk, assert no panic and
  the line is dropped); broken-pipe → synthetic error.
- **`crew-echo-plugin`:** a small unit/integration test of its command→event mapping.
- **`ChatPane`:** unit-test `cells()` layout — given messages + input + (cols,rows),
  assert the input line and a wrapped message land at expected cells; and the input
  edit reducer (push/backspace/enter-clears).
- **`crew-app`:** existing terminal tests stay green; a `PaneContent` smoke test that
  a chat pane produces cells without a real plugin (inject a fake).
- GPU/launch gate unchanged (compile + clippy + tests + timeboxed non-panic launch).

---

## 11. Open questions (resolve in planning, not blocking)

1. Exact `Cmd+J` vs another free chord; whether channel-switching gets a key in v1
   (likely defer — one channel from the echo plugin).
2. Whether `crew-plugin` is a new crate or a `crew-app` module — leaning crate (clean
   boundary, lets the echo plugin share the protocol types).
3. Message history cap per chat pane (ring buffer size) before older lines drop.
4. Whether `ChatPane::cells` shares wrapping helpers with the terminal cell layout or
   has its own (likely its own — different layout entirely).
