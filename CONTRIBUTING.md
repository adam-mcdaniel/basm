# Contributing to `basm`

Welcome! 🎉 Thank you for your interest in contributing to `basm`, a Brainfuck assembler designed for efficiency and usability. Whether you're fixing a bug, adding features, improving documentation, or suggesting ideas, your contributions are valuable.

## Table of Contents
- [How to Contribute](#how-to-contribute)
- [Reporting Issues](#reporting-issues)
- [Submitting Changes](#submitting-changes)
- [Code Guidelines](#code-guidelines)
- [Testing](#testing)
- [Project Structure](#project-structure)
- [Getting Help](#getting-help)

---

## How to Contribute

1. **Fork the repository** on GitHub.
2. **Clone your fork** to your local machine:
````sh
git clone https://github.com/your-username/basm.git
````
3. **Create a new branch** for your changes:
```sh
git checkout -b my-feature-branch
```
4. **Make your changes** following the [Code Guidelines](#code-guidelines).
5. **Test your changes** to ensure they work as expected (see [Testing](#testing)).
6. **Commit your changes** with a clear message:
```sh
git commit -m "Add feature X"
```
7. **Push to your fork**:
```sh
git push origin my-feature-branch
```
8. **Submit a pull request** to the main repository.

## Reporting Issues
If you encounter a bug, inconsistency, or want to request a feature:
1. Check existing issues to avoid duplicates.
2. Open a new issue with:
    - A clear and descriptive title.
    - Steps to reproduce (if applicable).
    - Expected behavior vs. actual behavior.
    - Error messages or logs (if any).

Feature requests should include:
- The problem the feature would solve.
- A proposed implementation (if possible).
- Any alternatives considered.

## Submitting Changes
Before submitting a PR:
1. Ensure your code follows the [Code Guidelines](#code-guidelines).
2. **Write tests** for new features or bug fixes.
3. **Update documentation** if necessary.
4. **Keep commits clean and atomic** — each commit should represent a single logical change with a clear purpose.
6. **Describe your changes** clearly in the PR.

## Code Guidelines

- Follow Rust's idioms.
- Use `cargo fmt` to format your code.
- Use meaningful variable and function names.
- Keep functions small and focused.
- Comment non-trivial code to explain the logic.
- Write modular and reusable code.

## Testing

Use `cargo test` to run the test suite. Ensure all tests pass before submitting a PR.

### Writing Tests

- Every new feature should have corresponding tests.
- Tests should cover edge cases and error conditions.

## Project Structure
- `src/`: Source code for the assembler.
    - `lib.rs`: Main library file.
    - `asm/`: Assembly parsing and processing.
    - `bf/`: Brainfuck code generation.
    - `utils/`: Utility functions for ASCII manipulation and more.
        - `templates/`: ASCII art templates for BrainF*** code.
- `tests/`: Integration tests.
- `examples/`: Example assembly code.
- `benches/`: Benchmark tests.
- `etc/`: Additional resources and tools.
- `assets/`: Files for the [README](README.md).
- `Cargo.toml`: Project configuration.

## End notes

Happy coding!🚀  Your contributions make `basm` better. Thank you! ❤️