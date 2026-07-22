# Ground Control

Observe local agent sessions and build toward durable managed agent work.

Ground Control is a local control plane for agent work. Today it indexes and monitors Claude Code sessions; the first managed slice adds a persistent broker that can launch and control Codex work without tying it to a terminal or desktop window. It complements [Gantry](https://github.com/phatblat/gantry), which owns agent configuration packages.

Ground Control distinguishes three kinds of session:

- **Managed** — launched and controlled through a supported runtime adapter.
- **Cooperative** — externally launched, with only the controls the runtime explicitly exposes.
- **Observed** — local telemetry such as Claude transcripts and heartbeats; observation never implies control.

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

The current Rust workspace contains the observation baseline:

| Crate | Purpose |
|-------|---------|
| `gc-core` | Claude observation parser, SQLite index with FTS5 search, data models |
| `gc-cli` | `gc` binary — terminal interface |
| `ground-control` (src-tauri) | Tauri 2 desktop app |

Claude data flows from its configured local directory into the existing observation index at `~/.local/share/ground-control/index.db`. That legacy index is a derived cache and can be rebuilt with `gc index`.

The managed architecture adds `gc-brokerd` as the sole runtime and journal authority, `gc-protocol` as the shared bounded wire contract, and Codex App Server as the first managed adapter. Its event journal is authoritative; only cursor-qualified current-state views are rebuilt from it. The CLI and Tauri app become thin clients of the same broker protocol.

The first managed slice is headless and manually starts the broker. It uses the installed Codex defaults, a trusted executable outside the worktree, and a clean Git repository at an explicit commit. Tauri service integration, provider profiles, fallback, acceptance, and recovery briefs follow in later packages.

Public client data is metadata and redacted evidence summaries by default. Raw messages, prompts, tool arguments, patches, receipts, credentials, and broker capabilities do not enter the normal client surface or managed worktree environment.

See [docs/spec.md](docs/spec.md) for the full specification.

## Schemas

JSON Schemas for Claude Code's runtime data surfaces live in `schema/`:

- `session-registry.schema.json` — live session heartbeat files
- `session-entry.schema.json` — JSONL transcript entries (12-variant discriminated union)
- `history-entry.schema.json` — global prompt history

These are authored from observation of the local file format and will eventually be extracted to [claude-config-schema](https://github.com/phatblat/claude-config-schema).

## License

This repo is licensed under the MIT License. See the [LICENSE](LICENSE.md) file for rights and limitations.
