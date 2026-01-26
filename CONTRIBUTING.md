# Contributing to mdview

Thank you for your interest in contributing to mdview! This document provides guidelines and information for contributors.

## Table of Contents

1. [Getting Started](#getting-started)
2. [Development Setup](#development-setup)
3. [Code Style](#code-style)
4. [Pull Request Process](#pull-request-process)
5. [Reporting Issues](#reporting-issues)
6. [Feature Requests](#feature-requests)

---

## Getting Started

### Prerequisites

- Rust 1.75 or later (stable)
- Cargo (comes with Rust)
- Git

### Clone the Repository

```bash
git clone https://github.com/yourusername/mdview.git
cd mdview
```

### Build

```bash
# Development build
cargo build

# Release build
cargo build --release

# With all features
cargo build --all-features
```

### Run Tests

```bash
cargo test
```

---

## Development Setup

### Project Structure

```
mdview/
├── src/
│   ├── main.rs           # Entry point
│   ├── lib.rs            # Library exports
│   ├── app/              # Application state & viewer
│   ├── config/           # Configuration loading
│   ├── markdown/         # Markdown parsing & rendering
│   ├── toc/              # Table of contents
│   ├── annotations/      # Annotations system
│   ├── export/           # PDF export
│   ├── theme/            # Theme system
│   ├── watcher/          # File watching
│   └── plugin/           # Plugin system (feature-gated)
├── themes/               # Built-in themes
├── plugins/              # Example plugins
└── docs/                 # Documentation
```

### Feature Flags

```bash
# Default (includes syntax highlighting)
cargo build

# With plugins
cargo build --features plugins

# Minimal build
cargo build --no-default-features
```

---

## Code Style

### Rust Style

- Follow standard Rust conventions
- Use `rustfmt` for formatting: `cargo fmt`
- Use `clippy` for linting: `cargo clippy`
- Keep functions under 50 lines when possible
- Document public APIs with doc comments

### Naming Conventions

- **Structs/Enums**: PascalCase (`TocPanel`, `FileEvent`)
- **Functions/Methods**: snake_case (`load_file`, `render_heading`)
- **Constants**: SCREAMING_SNAKE_CASE (`MAX_CACHE_ENTRIES`)
- **Modules**: snake_case (`file_watcher`, `markdown`)

### Documentation

- Document all public items
- Include examples in doc comments where helpful
- Update `docs/` when adding features

---

## Pull Request Process

### Before Submitting

1. **Fork** the repository
2. **Create a branch** for your feature: `git checkout -b feature/my-feature`
3. **Make changes** following the code style guidelines
4. **Add tests** for new functionality
5. **Run tests**: `cargo test`
6. **Run formatter**: `cargo fmt`
7. **Run linter**: `cargo clippy`
8. **Commit** with descriptive messages

### Commit Messages

Use clear, descriptive commit messages:

```
feat: add syntax highlighting for Python

- Add Python syntax support via syntect
- Update theme colors for Python keywords
- Add tests for Python code blocks
```

Prefixes:
- `feat:` - New feature
- `fix:` - Bug fix
- `docs:` - Documentation
- `refactor:` - Code refactoring
- `test:` - Adding tests
- `chore:` - Maintenance tasks

### Submitting

1. Push to your fork
2. Open a Pull Request against `main`
3. Fill out the PR template
4. Wait for review

### Review Process

- At least one maintainer approval required
- All CI checks must pass
- Address review feedback promptly

---

## Reporting Issues

### Bug Reports

Include:
- **Description**: Clear description of the bug
- **Steps to Reproduce**: Numbered steps to reproduce
- **Expected Behavior**: What should happen
- **Actual Behavior**: What actually happens
- **Environment**: OS, Rust version, mdview version
- **Screenshots**: If applicable

### Template

```markdown
## Bug Description
[Clear description]

## Steps to Reproduce
1. Step one
2. Step two
3. Step three

## Expected Behavior
[What should happen]

## Actual Behavior
[What actually happens]

## Environment
- OS: [e.g., macOS 14.0]
- Rust: [e.g., 1.75.0]
- mdview: [e.g., 0.1.0]

## Additional Context
[Screenshots, logs, etc.]
```

---

## Feature Requests

### Proposing Features

1. **Search existing issues** to avoid duplicates
2. **Open an issue** with the `enhancement` label
3. **Describe the feature** clearly
4. **Explain the use case** - why is this needed?
5. **Consider implementation** - how might it work?

### Template

```markdown
## Feature Description
[Clear description of the feature]

## Use Case
[Why is this feature needed? What problem does it solve?]

## Proposed Implementation
[Optional: How might this be implemented?]

## Alternatives Considered
[Optional: Other approaches you've considered]
```

---

## Questions?

- Open a [Discussion](https://github.com/yourusername/mdview/discussions)
- Check existing [Issues](https://github.com/yourusername/mdview/issues)
- Read the [Documentation](docs/)

Thank you for contributing!
