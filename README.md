# Rust DJ Engine

A modular, high-performance Rust audio engine for DJ applications.

## Overview

This project implements a headless Rust library providing a reusable audio engine for DJ apps. It features runtime-selectable audio backends, modular decks, and configurable audio routing.

## Architecture

The project is organized as a Cargo workspace with the following crates:

- **audio-core**: Core audio traits and types
- **backend-null**: Null backend for testing and CI
- **backend-miniaudio**: Miniaudio backend (Sprint 1)
- **backend-cpal**: Cross-platform audio via CPAL (native PipeWire on Linux)
- **engine-core**: Main engine orchestration
- **engine-dsp**: Pure DSP components (decks, mixer, analyzers)
- **codec**: Audio decoding wrapper (Sprint 1)
- **resampler**: Audio resampling (Sprint 1)
- **library**: Library manager and metadata (Sprint 3)
- **app-example**: Example application

## Current Status

**Sprint 0 - Workspace & Scaffolding** ✅ **COMPLETED**

- ✅ Cargo workspace with all required crates
- ✅ audio-core trait and types implementation
- ✅ backend-null implementation with tests
- ✅ engine-dsp minimal deck and mixer stub
- ✅ Unit tests for DSP modules
- ✅ CI skeleton with GitHub Actions

## Quick Start

### Prerequisites

- Rust 1.70+ (stable, beta, or nightly)
- Linux x86_64 (primary development platform)

### Building

```bash
# Clone the repository
git clone <repository-url>
cd rust-dj-engine

# Build all crates
cargo build

# Run tests
cargo test

# Run the example application
cargo run --bin rust-dj-engine-example
```

### Running Tests

```bash
# Run all tests
cargo test

# Run tests for specific crate
cargo test --package audio-core
cargo test --package backend-null
cargo test --package engine-dsp

# Run tests with output
cargo test -- --nocapture
```

## Development

### Code Style

The project uses:

- `cargo fmt` for formatting
- `cargo clippy` for linting
- Standard Rust naming conventions

### Testing

- Unit tests are included in each crate
- Integration tests use the null backend
- CI runs tests on multiple Rust versions

### CI/CD

GitHub Actions workflow includes:

- Format checking
- Clippy linting
- Testing on stable, beta, and nightly Rust
- Security auditing
- Documentation generation

## Roadmap

### Sprint 1 - Miniaudio & Codec (Next)

- [ ] Implement backend-miniaudio
- [ ] Implement codec wrapper (symphonia)
- [ ] Implement resampler abstraction and rubato impl
- [ ] Add TOML config parsing

### Sprint 2 - Producer/Consumer Plumbing

- [ ] Implement ring buffer + producer thread
- [ ] Integrate producer -> resampler -> ring buffer -> consumer render
- [ ] Add device + channel mapping logic

### Sprint 3 - Library Manager & Tags

- [ ] Add library crate with tag reading
- [ ] Implement SQLite schema
- [ ] Provide APIs for metadata management

### Sprint 4 - PipeWire & WASM Prototyping

- [ ] Prototype WASM build for engine-dsp

## License

GPL-3.0

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests
5. Run `cargo fmt` and `cargo clippy`
6. Submit a pull request

## Technical Details

See [docs/tech-spec.md](docs/tech-spec.md) for the complete technical specification.
