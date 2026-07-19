# Contributing

## Code of Conduct

Please be respectful and constructive. We follow the Rust Code of Conduct.

## Getting Started

1. Fork the repository
2. Set up the build environment (see Building from Source)
3. Make your changes
4. Run tests
5. Submit a pull request

## Code Style

- Follow Rust edition 2024 idioms
- Use `cargo fmt` before committing
- Use `cargo clippy` and address all warnings
- No `unsafe` code unless absolutely necessary and documented
- Async trait methods should use `#[async_trait]`

## Pull Request Guidelines

- Keep PRs focused on a single change
- Include tests for new functionality
- Update documentation (mdbook pages) for user-facing changes
- Ensure all existing tests pass
- Add a clear description of the change and motivation

## Commit Messages

Follow conventional commits:

```
feat: add feature X
fix: correct Y behavior
docs: update API reference
refactor: simplify Z
test: add tests for W
```

## Project Structure

```
├── crates/
│   ├── ofs-core/          # Domain types and traits
│   ├── ofs-proto/         # Protobuf definitions
│   ├── ofs-registry/      # SQLite registry
│   ├── ofs-offline-store/ # DuckDB offline store
│   ├── ofs-online-store/  # SQLite/Redis online store
│   ├── ofs-materialization/ # Materialization engine
│   └── ofs-python/        # Python bindings
├── protos/                # Protobuf source files
├── docs/                  # mdbook documentation
└── scripts/               # Build and utility scripts
```
