# Contributing to JetStreamProto

Thank you for your interest in contributing to JetStreamProto! This document provides guidelines and instructions for contributing.

## Code of Conduct

Please be respectful and constructive in all interactions. We aim to maintain a welcoming and inclusive community.

## How to Contribute

### Reporting Bugs

1. Check if the bug has already been reported in [Issues](https://github.com/yourusername/JetStreamProto/issues)
2. If not, create a new issue with:
   - Clear title and description
   - Steps to reproduce
   - Expected vs actual behavior
   - System information (OS, Rust version, etc.)
   - Code samples if applicable

### Suggesting Features

1. Check [Discussions](https://github.com/yourusername/JetStreamProto/discussions) for similar suggestions
2. Create a new discussion or issue describing:
   - The problem you're trying to solve
   - Your proposed solution
   - Alternative approaches considered
   - Potential impact on existing functionality

### Pull Requests

1. **Fork** the repository
2. **Create a branch** from `main`:
   ```bash
   git checkout -b feature/your-feature-name
   ```
3. **Make your changes**:
   - Follow the coding standards below
   - Add tests for new functionality
   - Update documentation as needed
4. **Test your changes**:
   ```bash
   cargo test --workspace
   cargo clippy --workspace
   cargo fmt --check
   ```
5. **Commit** with clear messages:
   ```bash
   git commit -m "feat: add new feature X"
   ```
6. **Push** to your fork:
   ```bash
   git push origin feature/your-feature-name
   ```
7. **Open a Pull Request** with:
   - Clear description of changes
   - Reference to related issues
   - Screenshots/examples if applicable

## Development Setup

### Prerequisites

- Rust 1.70 or later
- Cargo
- Git

### Building

```bash
# Clone your fork
git clone https://github.com/yourusername/JetStreamProto.git
cd JetStreamProto/jetstream_proto

# Build
cargo build

# Run tests
cargo test --workspace

# Run specific package tests
cargo test -p jsp_transport
```

### Running Examples

```bash
# Chat server
cargo run --bin chat_server

# Chat client
cargo run --bin chat_client

# Benchmarks
cargo bench -p jsp_benchmarks
```

## Coding Standards

### Rust Style

- Follow [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Use `cargo fmt` for formatting
- Use `cargo clippy` for linting
- Maximum line length: 100 characters

### Documentation

- Add doc comments for all public APIs
- Include examples in doc comments
- Update relevant documentation files

### Testing

- Write unit tests for new functionality
- Add integration tests for complex features
- Ensure all tests pass before submitting PR
- Aim for >80% code coverage

### Commit Messages

Follow [Conventional Commits](https://www.conventionalcommits.org/):

- `feat:` new feature
- `fix:` bug fix
- `docs:` documentation changes
- `test:` test additions/changes
- `refactor:` code refactoring
- `perf:` performance improvements
- `chore:` maintenance tasks

Example:
```
feat: add support for WebRTC transport

- Implement WebRTC data channel transport
- Add configuration options
- Include integration tests
- Update documentation

Closes #123
```

## Project Structure

```
jetstream_proto/
├── jsp_core/           # Core protocol definitions
├── jsp_transport/      # Transport layer implementation
├── jsp_gateway/        # Gateway and load balancer
├── jsp_python/         # Python bindings
├── jsp_wasm/          # JavaScript/WASM bindings
├── jsp_benchmarks/    # Performance benchmarks
├── jsp_integration_tests/ # Integration tests
├── jetstream_examples/    # Example applications
└── docs/              # Documentation
```

## Review Process

1. **Automated Checks**: CI will run tests and linting
2. **Code Review**: Maintainers will review your code
3. **Feedback**: Address any requested changes
4. **Approval**: Once approved, PR will be merged

## Questions?

- Open a [Discussion](https://github.com/yourusername/JetStreamProto/discussions)
- Check existing [Issues](https://github.com/yourusername/JetStreamProto/issues)
- Read the [Documentation](docs/)

## License

By contributing, you agree that your contributions will be licensed under the MIT License.

---

Thank you for contributing to JetStreamProto! 🚀
