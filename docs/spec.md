# Ground Control

Ground Control is a local control plane for agent work. It observes sessions that expose only telemetry and, when a runtime offers a supported control surface, launches and manages work without tying its lifetime to a terminal or desktop window.

It complements [Gantry](https://github.com/phatblat/gantry): Gantry prepares agent configuration packages; Ground Control owns runtime observation, orchestration policy, provider profiles, operational evidence, and controls.

The name follows the launch metaphor: Gantry prepares the vehicle, while Ground Control monitors and directs the mission.

## Product boundary

Ground Control treats sessions according to the authority it actually has:

| Session class | Ground Control can observe | Ground Control can control |
|---|---|---|
| Managed | Runtime state, usage, evidence, and freshness | Launch and supported lifecycle actions through a managed adapter |
| Cooperative | Telemetry voluntarily exposed by an external runtime | Only actions explicitly exposed by that runtime |
| Observed | Local files and process metadata | Nothing; a PID or transcript never implies control |

The first managed runtime is the installed Codex App Server. Existing Claude Code sessions remain observed until Claude exposes a supported control surface. Ground Control never presents an observed PID as a controllable process.

## Core vocabulary

- **Work unit:** The operator's durable objective and acceptance policy.
- **Attempt:** One execution of a work unit on a specific runtime, provider, host, and isolated workspace.
- **Turn:** One provider-recognized interaction inside an attempt.
- **Event journal:** Append-only private operational history owned by Ground Control.
- **Current-state view:** State deterministically reduced from journal events through a named journal sequence and reducer version.
- **Fixture replay:** Scrubbed provider or collector records passed through real ingestion and reduction code to verify canonical output.
- **Evidence:** A provenance-bearing observation or verification result. An agent's final message is evidence, not proof that acceptance passed.
- **Recovery brief:** Deterministic guidance that distinguishes confirmed, attempted, and unknown effects before retrying work.

## Architecture

Ground Control is moving from several process-local observers to one persistent per-user service:

| Component | Responsibility |
|---|---|
| `gc-core` | Provider-neutral domain types, Claude observation, journal, migrations, reducers, and current-state views |
| `gc-protocol` | Versioned, bounded, redacted broker wire types and reusable client |
| `gc-brokerd` | Sole journal writer, runtime owner, policy authority, and command coordinator |
| `gc` | Reference headless client for observation and managed controls |
| Tauri app | Thin presentation client for the same broker snapshots, follow stream, and commands |
| Runtime adapters | Translate runtime-specific identities, capabilities, events, and lifecycle actions |

Clients do not gain authority by reading SQLite directly. They request an atomic snapshot at a current-state cursor, follow journal sequences inclusively, deduplicate events, and resnapshot when compatibility or cursor rules require it.

### Data authority

Two different storage promises coexist during migration:

- `~/.local/share/ground-control/index.db` is the existing derived Claude observation cache. It may be rebuilt from retained Claude source files.
- The managed control-plane database is authoritative. Its append-only event journal, command intent, evidence, and uncertainty cannot be discarded as a cache. Current-state views are rebuildable from that journal.

The legacy index remains a read-only import source until explicitly retired. Aggregate legacy rows are labeled observations; they are never converted into fabricated turns or verified evidence.

### Failure semantics

Ground Control records broker-to-runtime command dispatch separately from runtime-to-provider request state. A runtime write failure, ambiguous provider send, interrupted stream, provider-native retry, and completed result are different states. Unknown outcomes are quarantined instead of replayed blindly.

The Ground Control journal is the durability authority. Stable workflow steps, durable due times, command intent, dispatch start, acknowledgement, and outcome state let the broker recover after restart without making an agent SDK or external workflow engine a second source of truth.

## Current observation surface

The shipped CLI observes Claude Code data from its configured local directory:

```text
gc index                      Scan Claude data and rebuild the observation index
gc list [--project <name>]    List observed sessions
gc search <query>             Search observed session metadata
gc burn                       Summarize observed token usage
gc live                       Show current observed session heartbeats
```

Claude's local formats are undocumented and may change. Parsers therefore preserve deterministic source identity, process only complete records, and surface unknown or invalid variants as diagnostics. They do not infer paths by reversing ambiguous hyphen encoding, invent identities, or silently treat unknown input as understood.

JSON Schemas for observed formats live in `schema/`. Scrubbed and synthetic fixture replay protects the ingestion contract without committing local session content.

## First managed vertical slice

The first slice is deliberately headless:

1. Align the durable product contract and repair the Claude observation baseline.
2. Introduce ordered migrations, the private journal, deterministic reducers, and fixture replay.
3. Establish the versioned protocol and manually started `gc-brokerd` as the sole writer and runtime authority.
4. Supervise one installed Codex App Server per attempt and negotiate capabilities truthfully.
5. Complete create, launch, inspect, follow, steer, interrupt, exact approval, and cancel through the CLI.
6. Prove restart-safe coordination and an isolated workspace seeded from a clean Git repository at an explicit commit.

The slice uses the installed Codex default configuration. Ground Control records the trusted absolute executable path and fingerprint plus the resolved runtime, provider, model, and configuration fingerprint. Provider profiles, fallback, side-effect recovery, acceptance runners, the Tauri cutover, and macOS LaunchAgent integration remain later packages.

Managed App Server children receive no broker credential in this slice. The private operator endpoint derives authority from its owner-only local connection; client-supplied role fields are never authoritative.

## Privacy and security posture

- Operational data stays local and private by default.
- Public client DTOs are explicit metadata and evidence-summary allowlists. V1 has no client operation for raw frames, full messages, prompts, tool arguments, patches, or receipts.
- Credentials belong in macOS Keychain behind origin-bound opaque references and must not enter argv, journals, logs, diagnostics, evidence, or recovery briefs.
- Managed work runs in isolated worktrees. Broker credentials, privileged descriptors, and operator state stay outside the worktree and child environment.
- Unix-socket permissions and peer UID reduce accidental privilege spread; they are not a hard boundary against malicious unsandboxed code running as the same user.
- Token budgets are optional and advisory. Missing usage remains unknown rather than zero and never blocks launch or acceptance.

## Relationship to Gantry

| Concern | Gantry | Ground Control |
|---|---|---|
| Primary focus | Agent configuration content | Runtime observation and control |
| Timing | Before and between sessions | During and after work |
| Owned data | Instructions, hooks, skills, rules, settings | Work units, attempts, runtime profiles, policy, evidence, usage, and effects |
| Actions | Lint, scaffold, snapshot, and edit configuration | Observe, launch, control, recover, and verify supported runtimes |

Ground Control may report which Gantry configuration was active and may propose a missing prerequisite. It does not silently install plugins or edit Gantry-owned configuration. Gantry does not become the runtime process or evidence authority.

## Product milestones

### Observation baseline

- Claude session indexing, listing, search, token burn, and live heartbeat views
- SQLite FTS5 observation index
- Tauri/Svelte application shell
- Local schemas for observed Claude formats

### Managed headless slice

- Correctness fixtures and visible ingestion diagnostics
- Authoritative journal and deterministic current-state views
- `gc-brokerd`, shared protocol, and CLI cutover
- Managed Codex adapter with capability negotiation
- Restart-safe work-unit lifecycle and clean immutable workspace seed

### Desktop and recovery

- Tauri broker client and per-user macOS service lifecycle
- Attention-oriented mission board and exact controls
- Provider profiles, Keychain-backed secrets, evidence, effects, and acceptance
- Checkpoint recovery, configured fallback, and “Retry with recovery brief”
- Production packaging, upgrades, diagnostics, storage health, and explicit purge

## Non-goals

- **Not a provider chat client.** Ground Control presents operational state, evidence, and controls rather than replacing a runtime's conversational UI.
- **Not an authority over observed sessions.** Filesystem telemetry and PIDs do not confer control.
- **Not an agent configuration editor.** Gantry continues to own configuration packages.
- **Not a promise of external rollback.** Retry and recovery preserve and communicate uncertainty; they cannot undo arbitrary real-world effects.
- **Not a cloud scheduler in v1.** The initial authority is a private per-user service on one Mac.
- **Not an external workflow-runtime integration.** The GC journal and coordinator own v1 durability; another runtime requires a separate post-v1 adoption decision.

## Implementation choices

| Layer | Choice | Rationale |
|---|---|---|
| Core and broker | Rust | Shared types, explicit errors, and reliable local process ownership |
| Database | SQLite via bundled `rusqlite` | Transactional local authority and deterministic rebuildable views |
| Observation | `notify` plus fixture-backed parsers | Native filesystem events with reproducible ingestion tests |
| CLI | `clap` | Typed headless reference client |
| Desktop | Tauri 2 and Svelte 5 | Small native shell over the shared broker protocol |
| Broker transport | Private Unix-domain socket | Bounded local protocol with peer identity and no LAN listener |

## Open questions

1. Which minimal ServiceManagement bridge best fits the final Tauri bundle and signing layout?
2. Which optional Codex override and exact-turn fork capabilities are supported by each installed App Server version?
3. What bounded checkpoint limits are safe after fixture and stress testing?
4. When, if ever, does an external durability SDK justify adding a second runtime boundary after v1?
