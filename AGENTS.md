# Repository Guidelines

## Project Structure & Module Organization

- `crates/gc-core/`: shared Rust library for Claude session parsing, indexing, models, and live-session watching.
- `crates/gc-cli/`: `gc` command-line binary.
- `src-tauri/`: Tauri 2 desktop shell and Rust commands.
- `src/`: Svelte 5 frontend (`App.svelte`, `main.ts`).
- `schema/`: JSON Schemas for observed Claude Code data formats.
- `docs/`: project specs and implementation plans.
- `src-tauri/icons/`: desktop app assets.

## Build, Test, and Development Commands

Use `rtk` before shell commands, per `RTK.md` (for example, `rtk cargo test --workspace`).

- `just build`: build all Rust crates.
- `just test`: run `cargo test --workspace`.
- `just lint`: run Clippy across the workspace with warnings denied.
- `just check`: run Rust type checks without building artifacts.
- `just format`: run `cargo fmt --all` and format the `justfile`.
- `npm run check`: run Svelte and TypeScript checks.
- `npm run dev`: start the Vite frontend dev server.
- `cargo tauri dev` or `just dev`: run the desktop app locally.
- `cargo install --path crates/gc-cli`: install `gc`.

## Coding Style & Naming Conventions

Rust uses edition 2024 and standard `rustfmt` formatting. Keep modules focused and prefer explicit error handling with `anyhow` in binaries and typed errors in library code where useful. Name CLI commands and flags after user-facing behavior (`list`, `search`, `burn`, `live`). Svelte/TypeScript uses strict checking; keep component state typed and colocate UI-only helpers in the component.

## Testing Guidelines

Rust is the primary tested surface. Add unit tests near the module under test or integration tests under `crates/<crate>/tests/` when behavior crosses module boundaries. There is no dedicated npm test script, so use `npm run check` for frontend validation. Before handing off code changes, run `just test`, `just lint`, and `npm run check` when affected areas warrant it.

## Commit & Pull Request Guidelines

Recent history uses concise conventional-style subjects such as `feat: v0.2 live session monitoring`, `fix: include cache tokens in burn and list output`, and `docs: add project overview and usage to README`. Keep commits logically scoped and use lowercase prefixes like `feat:`, `fix:`, `docs:`, `chore:`, or `lock:`. PRs should include a summary, linked issue or plan, verification commands, and screenshots for visible Tauri/Svelte changes.

## Security & Configuration Tips

Do not commit local Claude session data, generated indexes, secrets, or machine-specific paths. The SQLite index is derived cache data; rebuild it with `gc index` rather than versioning it.
