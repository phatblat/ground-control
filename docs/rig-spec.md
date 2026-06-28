# rig

A CLI and TUI for inspecting, validating, editing, snapshotting, and reshaping Claude Code configuration.

`rig` manages the arrangement of an agent environment: `CLAUDE.md`, settings, hooks, skills, sub-agents, MCP servers, permissions, memory, and related configuration files. It complements Ground Control, which observes runtime behavior. The boundary is simple: rig manages configuration before and between sessions; Ground Control monitors sessions during and after execution.

## Problem

Claude Code configuration is powerful but spread across many files, directories, and implicit conventions. A mature setup can include global and project instructions, settings files, hook definitions, custom skills, sub-agents, MCP servers, permissions, output styles, and plugins. These pieces interact, but today there is no dedicated tool for answering basic operational questions:

- What is installed in this rig?
- Is it valid against the current Claude Code configuration formats?
- Which hooks, agents, skills, and MCP servers are active?
- What changed since the last known-good setup?
- Can I temporarily disable a component without deleting it?
- Can I snapshot and restore configuration safely?
- Which profile am I editing when `CLAUDE_CONFIG_DIR` is set?

Without a tool, developers inspect files manually, rely on memory, and debug failures by commenting out config fragments. That does not scale once a rig includes multiple hooks, skills, agents, and external servers.

## Goals

`rig` should make a Claude Code configuration legible, validatable, and safely editable.

The first milestone is intentionally read-only. Before rig writes configuration, it must be trusted to discover the active config root, enumerate components, validate them using shared schemas, and explain problems clearly.

Longer term, rig should provide a reversible editing workflow: snapshot before changes, edit with the user's editor, stow components without deleting them, hoist them back, and audit external changes.

## Non-goals

- **Not a runtime dashboard.** Session history, token burn, live processes, and transcript search belong to Ground Control.
- **Not a Claude Code replacement.** rig manages files consumed by Claude Code. It does not run agent sessions itself.
- **Not a schema authority.** Validation types come from `claude-config-schema`; rig should not fork or hand-maintain schemas that belong there.
- **Not a cloud sync product.** v0.x is local-first. Export/import can support portability later, but hosted account sync is out of scope.
- **Not an archetype classifier first.** Archetype detection is useful only after empirical validation against real rigs. It should not be a headline feature until the categories are proven.

## Architecture

`rig` is a Go single binary with both CLI and TUI surfaces.

| Layer | Choice | Rationale |
|-------|--------|-----------|
| Language | Go | Fast single-binary distribution, good filesystem/process ergonomics, strong fit for CLI tooling |
| CLI | Cobra or urfave/cli | Conventional subcommand UX; final choice should favor simple testability |
| TUI | Bubble Tea | Mature Go TUI framework; works over SSH; keeps rig terminal-native |
| Editing | `$EDITOR` | Users keep their editor workflow; rig validates and snapshots around edits |
| Validation | `claude-config-schema` Go bindings | Shared schema source of truth; avoids duplicate validation logic |
| Snapshots | Git repo at `~/.rig/repo` | Proven diff/history/revert model; no custom snapshot engine |
| Config discovery | `CLAUDE_CONFIG_DIR` first, then default locations | Required for profiles and nonstandard Claude Code installations |

### Package layout

```text
cmd/rig/                    CLI entrypoint
internal/configroot/         Config root discovery and profile resolution
internal/inventory/          Component discovery and normalization
internal/validate/           Schema validation integration
internal/snapshot/           Git-backed snapshot store
internal/stow/               Disable/restore primitives
internal/hooks/              rig-owned hook install/uninstall logic
internal/archetype/          Scoring model, initially experimental
internal/tui/                Bubble Tea screens and models
internal/report/             Human and machine-readable output
```

Public Go packages should be avoided until a real embedding use case appears. Early stability should be in the CLI contract and file formats, not in a Go library API.

## Configuration model

rig operates on a resolved config context:

```text
ConfigContext
  active_root        resolved CLAUDE_CONFIG_DIR or default Claude Code config dir
  scope              global, project, or explicit path
  project_root       optional current project root
  components         normalized inventory of discovered config components
  schema_version     claude-config-schema version used for validation
```

The active root must be explicit in all destructive or write-capable commands. If `CLAUDE_CONFIG_DIR` is set, rig should display it prominently in `doctor`, `ls`, and the TUI dashboard.

### Component types

rig should discover and normalize at least:

- Instructions: `CLAUDE.md`, included files, memory files
- Settings: user, project, and local settings JSON
- Hooks: Claude Code hooks and rig-installed hooks
- Skills: skill directories and frontmatter
- Agents: sub-agent markdown files and frontmatter
- MCP servers: `.mcp.json` and related server definitions
- Plugins: manifests, hooks, marketplace metadata when present
- Output styles and LSP config
- Permissions and tool allow/deny rules

Each component should retain source path, scope, parsed metadata, validation status, dependencies, and whether rig believes it owns or merely observes the file.

## CLI surface

```text
rig doctor
rig ls [--json] [--scope global|project|all]
rig lint [--json] [--strict]
rig diff [--from <snapshot>] [--to <snapshot|working>]
rig snapshot create [--message <text>]
rig snapshot list
rig snapshot show <id>
rig snapshot diff <id> [<id|working>]
rig snapshot revert <id>
rig edit <component>
rig stow <component>
rig hoist <component>
rig install [--hooks]
rig uninstall [--hooks]
rig export [--format tar|zip]
rig import <archive>
rig archetype [--experimental] [--json]
```

### v0.1 commands

v0.1 should ship only read-only commands:

```text
rig doctor
rig ls
rig lint
```

`doctor` checks environment and discovery. `ls` inventories components. `lint` validates against `claude-config-schema` and reports schema violations plus rig-specific warnings.

Write commands should wait until snapshot behavior is implemented and tested.

## TUI surface

The TUI is a convenience layer over the same core operations as the CLI. It should not contain separate business logic.

Initial screens:

- **Dashboard.** Active config root, health summary, component counts, most recent snapshot, schema version.
- **Components.** Browsable inventory grouped by type and scope. Opens selected files in `$EDITOR`.
- **Problems.** Validation errors and warnings grouped by file with exact paths.
- **Snapshots.** Timeline of git-backed snapshots with diff and revert affordances.

Later screens:

- **Dependency graph.** ASCII or braille graph in terminal; `rig graph --open` can launch a richer HTML view if needed.
- **Archetype advisor.** Experimental scoring view after empirical validation.

## Snapshots

Snapshots are stored in a managed git repository at `~/.rig/repo`.

The snapshot store should record selected config files from the active config root, preserving enough path context to restore safely. It should not blindly copy all of `~/.claude`; transcripts, caches, and secrets must be excluded.

Snapshot metadata should include:

- Snapshot id
- Timestamp
- Active config root
- Project root, if any
- Claude Code version, if detectable
- rig version
- Message
- File list and checksums

Before any write command, rig should create an automatic snapshot unless the user explicitly disables it for that command.

Open design point: whether snapshot storage should be one repo with paths namespaced by config root, or one repo per config root. One repo is simpler for history browsing; per-root repos reduce accidental cross-profile coupling.

## Stow and hoist

`stow` disables a component without deleting it. `hoist` restores it.

This should be implemented in a way that preserves original content and allows easy diffing. Candidate strategies:

- Move files to a `.rig/stowed/` directory under the config root.
- Rename files with a disabled suffix.
- Edit an owning manifest where one exists.

The default should be format-aware. For standalone files such as a skill or agent, moving to `.rig/stowed/` is likely safest. For settings-owned hook entries, rig should edit the owning JSON with a reversible marker.

Every stow/hoist action should snapshot first and record metadata explaining what changed.

## rig-owned hooks

`rig install` can install optional hooks that improve observability and safety:

- `PreToolUse` for activity and risk visibility
- `InstructionsLoaded` for config context capture
- `FileChanged` for external config mutation detection
- `ConfigChange` for auto-snapshot and audit

These hooks must be clearly marked as rig-owned in settings so `rig uninstall` can remove only what it owns. rig must not rewrite or reorder unrelated user hooks unless needed for valid JSON formatting.

Hook installation is not part of v0.1.

## Archetypes

The archetype model is useful as a diagnostic lens, but it is not yet validated. Treat it as experimental until tested against real configurations.

Proposed archetypes:

- Minimalist
- Autopilot
- Scholar
- Operator
- Craftsman
- Sentinel
- Conductor
- Fortress

Scoring dimensions:

- Autonomy
- Guardrails
- Specialization
- Team scale

Before making this prominent, collect roughly 20 to 30 real `.claude/` configurations, score them manually and automatically, and check whether the categories are emergent or merely invented. Until then, gate the command behind `--experimental`.

## Relationship to claude-config-schema

`claude-config-schema` is a foundational dependency. rig should consume its Go bindings for schema-backed parsing and validation.

The dependency direction is one-way:

```text
claude-config-schema -> rig
```

If rig discovers missing schemas or incorrect validation behavior, the fix belongs upstream in `claude-config-schema` unless it is a rig-specific rule.

Examples of rig-specific rules:

- Duplicate component names across scopes
- Hook command points at a missing executable
- MCP server command is not available on PATH
- Skill or agent exists but is unreachable due to directory layout
- A stowed component is referenced from active config

Examples that belong in `claude-config-schema`:

- Wrong JSON field type
- Missing required field
- Invalid enum value
- Agent or skill frontmatter shape
- Plugin manifest structure

## Relationship to Ground Control

Ground Control observes runtime sessions. rig manages configuration.

| Concern | rig | Ground Control |
|---------|-----|----------------|
| Focus | Configuration | Runtime sessions |
| Timing | Before and between sessions | During and after sessions |
| Data | Settings, hooks, skills, agents, MCP, instructions | Session registry, JSONL transcripts, token burn |
| Writes | Config files it owns or edits by command | No config writes |
| Output | Health, diffs, snapshots, editable config | Search, live status, metrics, resume |

Potential integration should stay contextual:

- Ground Control may show which rig snapshot was active during a session.
- rig may link to Ground Control sessions that used a component.
- Both may consume schemas from `claude-config-schema`.

Neither project should absorb the other.

## Milestones

### v0.1 - Read-only CLI

- [ ] Repo scaffold with Apache-2.0 license
- [ ] Config root discovery with `CLAUDE_CONFIG_DIR` support
- [ ] Component inventory model
- [ ] `rig doctor`
- [ ] `rig ls`
- [ ] `rig lint`
- [ ] Go binding integration with `claude-config-schema`
- [ ] JSON output for CI and scripting
- [ ] Fixture-based tests for common config layouts

### v0.2 - Snapshots and diffs

- [ ] Managed git snapshot store
- [ ] `rig snapshot create/list/show/diff/revert`
- [ ] `rig diff`
- [ ] Snapshot metadata format
- [ ] Secret and cache exclusion rules

### v0.3 - Safe writes

- [ ] `rig edit`
- [ ] `rig stow`
- [ ] `rig hoist`
- [ ] `rig install`
- [ ] `rig uninstall`
- [ ] Auto-snapshot before writes
- [ ] Ownership markers for rig-managed config fragments

### v0.4 - TUI

- [ ] Bubble Tea shell
- [ ] Dashboard
- [ ] Components browser
- [ ] Problems view
- [ ] Snapshot timeline

### v0.5 - Advanced analysis

- [ ] Dependency graph
- [ ] Experimental archetype scoring
- [ ] Export/import
- [ ] HTML graph view
- [ ] Distribution via Homebrew and mise

### v1.0 - Stable local configuration manager

- [ ] Stable CLI contract
- [ ] Stable snapshot format
- [ ] Documented recovery procedures
- [ ] Validated archetype model or explicit removal from headline UX
- [ ] Polished install/update/uninstall flow

## Open decisions

These need to be settled before or during v0.1.

1. **Repository home.** Personal repo or new organization. Personal is faster; an org better matches the broader ecosystem if `claude-config-schema` also moves there.
2. **License.** Apache-2.0 is recommended for ecosystem tooling and explicit patent terms. MIT is simpler but less protective.
3. **CLI framework.** Cobra has the largest ecosystem and familiar command shape; urfave/cli is smaller. Pick based on test ergonomics, not fashion.
4. **Snapshot namespace.** One global git repo under `~/.rig/repo` versus one repo per config root.
5. **Machine output contract.** Whether JSON output is best-effort in v0.x or treated as stable enough for CI from v0.1.
6. **Config layout fixtures.** Need representative real-world `.claude/` configs for tests, scrubbed of secrets.
7. **Stow mechanics.** Decide per component type: move, rename, or manifest edit.
8. **Hook ownership markers.** Define the exact metadata rig writes so uninstall is precise and reversible.
9. **Archetype validation threshold.** Decide how many real configs are enough before promoting archetypes from experimental.
10. **Distribution target.** Homebrew, mise, GitHub releases, or all three. This can wait until after v0.1 but affects repo setup.

## Risks

1. **Claude Code config formats are not fully stable.** Mitigation: rely on `claude-config-schema`, keep fixtures, and degrade gracefully where possible.
2. **Write commands can damage user workflow.** Mitigation: no writes before snapshots, exact ownership markers, clear dry-run output.
3. **Secrets may live near config.** Mitigation: conservative snapshot inclusion, explicit exclude rules, and warnings for likely secret files.
4. **Archetypes may be invented rather than discovered.** Mitigation: keep experimental until validated.
5. **Profiles can be confused.** Mitigation: make active config root visible everywhere and require confirmation for writes outside the expected root.

## First implementation slice

The first useful implementation should avoid the TUI and all writes.

1. Resolve active config root.
2. Walk known Claude Code config locations.
3. Normalize discovered files into component records.
4. Validate schema-backed files through `claude-config-schema`.
5. Emit human-readable and JSON reports.

That slice is enough to prove rig's foundation and unblock later snapshot/write work without risking user configuration.
