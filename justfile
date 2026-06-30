set export
set ignore-comments
set script-interpreter := ['bash', '-eu']
set quiet
set unstable

[script]
_default:
    just --list

#
# build recipes
#

# Build all Rust crates
[group('build')]
build:
    cargo build

# Build in release mode
[group('build')]
release:
    cargo build --release

# Run the CLI
[group('build')]
run *args:
    cargo run --bin gc -- {{ args }}

#
# checks recipes
#

# Run clippy lints
[group('checks')]
lint:
    cargo clippy --workspace -- -D warnings

# Check types without building
[group('checks')]
check:
    cargo check --workspace

#
# tests recipes
#

# Run all tests
[group('tests')]
test:
    cargo test --workspace

#
# configuration recipes
#

# Remove build artifacts
[group('configuration')]
clean:
    cargo clean
    rm -rf dist node_modules

# Install JS dependencies
[group('configuration')]
deps:
    npm install

# Format Rust code
[group('configuration')]
format:
    cargo fmt --all
    just --fmt

#
# tauri recipes
#

# Run Tauri dev server
[group('tauri')]
dev: deps
    cargo tauri dev

# Build Tauri app
[group('tauri')]
app: deps
    cargo tauri build

#
# data recipes
#

# Index all Claude Code sessions
[group('data')]
index:
    cargo run --bin gc -- index

# Show token burn summary
[group('data')]
burn:
    cargo run --bin gc -- burn

# Show live sessions
[group('data')]
live:
    cargo run --bin gc -- live
