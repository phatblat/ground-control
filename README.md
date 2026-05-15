# Ground Control

Monitor, manage, and search across all Claude Code sessions on your system.

Ground Control is the runtime dashboard for your Claude Code agents — it shows what they're doing, what they've done, and what they've cost. It complements [Gantry](https://github.com/phatblat/gantry), which manages agent configuration.

## Features

- **Cross-project session search** — find any session by title, agent name, or project without remembering which directory it was in
- **Token burn summary** — see exactly where tokens go, broken down by input (non-cached, cache read, cache new) and output
- **Live session monitoring** — see all running Claude Code sessions with PID, status, and working directory
- **Full-text search** — SQLite FTS5 index for fast search across all session metadata
- **Desktop app** — Tauri 2 app with Svelte 5 frontend (early stage)

## Install

Requires Rust 1.80+.

```sh
cargo install --path crates/gc-cli
```

## Usage

### Index sessions

Build the SQLite index from `~/.claude/` session data. Run this before other commands, and again when you want to pick up new sessions.

```
$ gc index
Indexed 15 sessions across 7 projects.
```

### List sessions

```
$ gc list
TITLE                                    PROJECT                  TOKENS     MSGS
----------------------------------------------------------------------------------
gc                                       control                   13.5M      290
wr phase 4                               rider                    164.7M     1368
claude-config-schema                     schema                    70.4M      914
...

$ gc list --project rider
```

### Search

```
$ gc search "refactor hook"
TITLE                                    PROJECT                  TOKENS
--------------------------------------------------------------------------
Handle duplicate hooks and file naming…  gantry                   323.0K
```

### Token burn

```
$ gc burn
Ground Control — Token Burn Summary
============================================
Sessions:                     15
Messages:                   3777
--------------------------------------------
Input (non-cached):         4.0K
Input (cache read):       278.0M
Input (cache new):         12.7M
Input total:              290.7M
--------------------------------------------
Output:                     1.1M
============================================
Total:                    291.8M
```

### Live sessions

```
$ gc live
PID      STATUS       CWD                            NAME
----------------------------------------------------------------
41343    idle         phatblat                       -
42377    idle         wave-rider                     wr phase 4
55164    busy         ground-control                 gc

3 live session(s)
```

## Desktop App

Early-stage Tauri 2 app with a Svelte 5 frontend. Requires the [Tauri CLI](https://v2.tauri.app/start/):

```sh
cargo install tauri-cli --version "^2"
npm install
cargo tauri dev
```

## Architecture

Rust workspace with three crates sharing a core library:

| Crate | Purpose |
|-------|---------|
| `gc-core` | JSONL parser, SQLite index with FTS5 search, data models |
| `gc-cli` | `gc` binary — terminal interface |
| `ground-control` (src-tauri) | Tauri 2 desktop app |

Data flows from Claude Code's local storage (`~/.claude/`) through a version-aware JSONL parser into a SQLite index at `~/.local/share/ground-control/index.db`. The index is a derived cache — it can be rebuilt from scratch at any time with `gc index`.

See [docs/spec.md](docs/spec.md) for the full specification.

## Schemas

JSON Schemas for Claude Code's runtime data surfaces live in `schema/`:

- `session-registry.schema.json` — live session heartbeat files
- `session-entry.schema.json` — JSONL transcript entries (12-variant discriminated union)
- `history-entry.schema.json` — global prompt history

These are authored from observation of the local file format and will eventually be extracted to [claude-config-schema](https://github.com/phatblat/claude-config-schema).

## License

This repo is licensed under the MIT License. See the [LICENSE](LICENSE.md) file for rights and limitations.
