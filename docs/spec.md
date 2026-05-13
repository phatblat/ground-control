# Ground Control

A desktop app for monitoring, managing, and searching across all Claude Code sessions on a system. Ground Control is the runtime dashboard — it shows what your agents are doing, what they've done, and what they've cost. It complements [Gantry](https://github.com/phatblat/gantry), which manages agent configuration; Ground Control manages agent observation.

The name follows a space launch metaphor: Gantry arranges the vehicle on the pad, Ground Control monitors the mission.

## Problem

Claude Code sessions accumulate across projects with no unified way to find, compare, or monitor them. The built-in `claude --resume` requires you to know which project directory a session belongs to. Token usage is invisible unless you scrape individual session files. There's no view of what's running across the system right now.

[Opcode](https://github.com/winfunc/opcode) addresses some of this but requires picking a project directory first, doesn't monitor live sessions, and ships under AGPL-3.0 which limits derivative use.

## Architecture

Rust workspace with three crates sharing a core library:

- **gc-core** — data models, JSONL parser, SQLite index with FTS5 search. Reads directly from Claude Code's local storage (`~/.claude/`). All parsing is behind a version-aware abstraction so format changes can be handled without rewriting consumers.
- **gc-cli** — `gc` binary for terminal use. Indexes sessions, searches, shows live status, reports token burn. Ships as a single binary with no runtime dependencies beyond SQLite (bundled).
- **ground-control** (src-tauri) — Tauri 2 desktop app. Rust backend calls gc-core; Svelte 5 frontend renders the UI. Communicates via Tauri commands (request/response) and Tauri events (streaming updates).

The SQLite index (`~/.local/share/ground-control/index.db`) is derived from the JSONL source files. It can be rebuilt from scratch at any time — it's a cache, not primary storage. FTS5 provides full-text search across session titles, agent names, and project paths.

### Data sources

Claude Code stores session data in `~/.claude/` across several file types:

| File | Location | Format | Purpose |
|------|----------|--------|---------|
| Session registry | `~/.claude/sessions/<pid>.json` | JSON | Live heartbeat for running sessions. Contains pid, status (idle/busy), cwd, version, kind (interactive/background), name. |
| Session transcript | `~/.claude/projects/<encoded-path>/<uuid>.jsonl` | JSONL | Append-only conversation history. Each line is a typed entry (user, assistant, system, attachment, etc.). Token usage embedded in assistant entries. Messages form a DAG via parentUuid. |
| Global history | `~/.claude/history.jsonl` | JSONL | Prompt history across all sessions. |

The JSONL transcript format is a discriminated union on the `type` field with 12 known variants: `user`, `assistant`, `attachment`, `system`, `agent-name`, `ai-title`, `custom-title`, `last-prompt`, `permission-mode`, `pr-link`, `queue-operation`, `file-history-snapshot`. JSON Schemas for these surfaces are maintained in `schema/` for eventual extraction to [claude-config-schema](https://github.com/phatblat/claude-config-schema).

Project directories use a path-encoding scheme where `/` is replaced with `-`, so `/Users/phatblat/dev/claude/ground-control` becomes `-Users-phatblat-dev-claude-ground-control`.

**Format stability risk.** None of this is a documented public API. The `version` field on JSONL entries (e.g. `2.1.140`) enables version-aware parsing. The parser should degrade gracefully on unknown fields (`additionalProperties: true` in schemas, `#[serde(deny_unknown_fields)]` not used in models) and skip entries it can't parse rather than failing the entire session.

## CLI surface

```
gc index                      Scan ~/.claude/ and rebuild the SQLite index
gc list [--project <name>]    List sessions, optionally filtered by project
gc search <query>             Full-text search across session titles and metadata
gc burn                       Token usage summary across all indexed sessions
gc live                       Show currently running sessions with PID/status/cwd
```

### Planned commands

```
gc resume [<query>]           Search + fzf picker → claude --resume in the right cwd
gc burn --weekly              Token burn broken down by week
gc burn --project <name>      Token burn scoped to a single project
gc watch                      Live-updating dashboard of running sessions
```

## Desktop app surface

The Tauri app provides what the CLI cannot: persistent visibility, real-time updates, and rich data visualization.

### Views

**Dashboard.** System-wide overview. Live session count with status indicators. Token burn chart (daily/weekly). Total sessions and projects. Rate limit status when available.

**Sessions.** Searchable, sortable table of all sessions across all projects. Columns: title, project, branch, status, token count, message count, timestamp. Click to expand session details. Resume button spawns `claude --resume` in the correct cwd via the user's terminal emulator.

**Live monitor.** Real-time view of active sessions. Watches `~/.claude/sessions/*.json` for status changes via the `notify` crate. Tails active JSONL files to show streaming activity. Shows current tool execution and token accumulation in progress.

**Project browser.** Grid of project cards showing session count, total tokens, recent activity. Click into a project to see its session list.

**Session detail.** DAG-aware conversation viewer. Since messages use `parentUuid` to form a tree (not a flat list), the viewer should render conversation forks as branches — similar to `git log --graph`. Shows token usage per turn, tool calls, and thinking blocks.

### System tray

Persistent tray icon showing active session count. Click to open the main window. Notifications when background agents complete or error.

## Tech stack

| Layer | Choice | Rationale |
|-------|--------|-----------|
| Core library | Rust | Shared between CLI and Tauri; fast JSONL parsing |
| Database | SQLite + FTS5 via rusqlite (bundled) | Cross-session search, metrics aggregation |
| File watching | notify crate (FSEvents on macOS) | Incremental JSONL indexing, live session monitoring |
| CLI | clap | Derive-based arg parsing |
| Desktop framework | Tauri 2 | ~3MB bundle, native webview, production-stable |
| Frontend | Svelte 5 (runes) | Lightest runtime, best DX, differentiates from Opcode's React |
| Components | shadcn-svelte | Professional component set |
| Charts | LayerCake or Chart.js | Token burn visualization |
| IPC | Tauri commands + events | Commands for request/response, events for streaming |

## Relationship to Gantry

| Concern | Gantry | Ground Control |
|---------|--------|----------------|
| Focus | Configuration of the agent | Runtime behavior of the agent |
| Timing | Before/between sessions | During/after sessions |
| Data | CLAUDE.md, hooks, skills, rules, settings | Sessions, conversations, tokens, metrics |
| Actions | Lint, scaffold, snapshot, edit config | Monitor, search, resume, analyze |

**Integration points.** Ground Control could display which Gantry archetype/configuration was active during each session. Gantry could link to Ground Control to show sessions that exercised a particular hook or skill. Both parse `~/.claude/` but for different purposes — they share the project path-encoding scheme and can share schema types via claude-config-schema.

**Boundary rule.** Ground Control never edits configuration. Gantry never displays session content. If a feature blurs this line, it belongs in whichever project owns the underlying data.

## Milestones

### v0.1 — CLI foundation (current)

- [x] JSONL parser with version-aware deserialization
- [x] SQLite index with FTS5 search
- [x] Rust data models for all 12 JSONL entry types
- [x] `gc index` — full reindex from ~/.claude/
- [x] `gc list` — cross-project session table
- [x] `gc search` — full-text search
- [x] `gc burn` — token usage summary
- [x] `gc live` — running session status
- [x] JSON Schemas for session registry, JSONL entries, history
- [x] Tauri app stub (compiles, wired to gc-core)
- [ ] `gc resume` with fzf integration and cwd navigation

### v0.2 — live monitoring

- [ ] File watcher for `~/.claude/sessions/` (detect new/changed/removed sessions)
- [ ] Incremental JSONL indexing (track byte offsets, tail new lines)
- [ ] `gc watch` command (live-updating terminal view)
- [ ] Tauri live session view with auto-refresh
- [ ] System tray with session count badge

### v0.3 — desktop app

- [ ] Svelte frontend: dashboard, sessions table, project browser
- [ ] Token burn charts (daily/weekly breakdown)
- [ ] Session detail view with conversation rendering
- [ ] `gc burn --weekly` and `gc burn --project`
- [ ] Proper app icon

### v0.4 — background agents

- [ ] Native support for `claude --bg` workflow
- [ ] Launch/attach/stop background sessions from UI
- [ ] Stream output from background sessions via `claude logs`
- [ ] Notifications on agent completion/error

### v0.5 — conversation intelligence

- [ ] DAG-aware conversation viewer (branch visualization)
- [ ] Full-text search across message content (not just titles)
- [ ] Session comparison (token efficiency across similar tasks)
- [ ] Export session summaries

### Future

- Cloud session support (when Anthropic exposes an API)
- Multi-machine session aggregation
- Gantry integration (show config active during each session)
- Agent remote control (inject prompts into running sessions)
- Cost estimation (token counts × model pricing)

## Non-goals

- **Not a session replay tool.** Ground Control indexes and searches sessions; it doesn't attempt to replay them interactively or provide an alternative chat interface.
- **Not a configuration manager.** That's Gantry's job. Ground Control reads config state for context but never writes it.
- **Not a Claude Code replacement.** Ground Control wraps `claude --resume` and `claude --bg` rather than reimplementing session management.
- **Not an Opcode fork.** Different architecture (Svelte vs React, CLI-first vs GUI-only), different license (MIT vs AGPL), different scope (live monitoring vs history browsing).

## Open questions

1. **Token cost estimation.** Should Ground Control maintain a pricing table for Anthropic models, or wait for Claude Code to expose cost data directly? Maintaining prices is manual and error-prone; waiting means no cost view until Anthropic adds it.
2. **Session pruning.** Should Ground Control offer to clean up old session data, or is that out of scope (too close to destructive operations on Claude Code's own storage)?
3. **JSONL content indexing.** FTS5 currently indexes titles and metadata. Indexing full message content would enable powerful search but significantly increases index size. Worth it?
4. **Resume UX.** When resuming a session, should Ground Control spawn a new terminal window, or attempt to reuse an existing one? Terminal emulator detection (`$TERM_PROGRAM`) is fragile.
5. **Shared schema dependency.** Should gc-core consume claude-config-schema as a crate dependency once Rust bindings are published, or maintain its own models? Own models are simpler but risk drift.
