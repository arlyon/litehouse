# Contributing to Litehouse

Thank you for your interest in contributing to Litehouse! We welcome contributions from everyone, and we are grateful for every pull request (PR) and issue report. This document provides guidelines for contributing to Litehouse.

## Code Style

We strive to maintain a consistent code style throughout the project. Please ensure your contributions adhere to the following guidelines:

- Use `rustfmt` to format your Rust code. You can run `cargo fmt` before submitting your PR.
- Follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/) for API design.
- Include comments in your code where necessary to explain complex logic.

## Pull Request Process

1. Fork the repository and create your branch from `main`.
2. If you've added code that should be tested, add tests.
3. Ensure the test suite passes by running `cargo test`.
4. Update the documentation if you have made changes to the API or added new features.
5. Submit your pull request with a clear description of the changes.

## Reporting Issues

When reporting issues, please provide as much detail as possible about the problem. Include steps to reproduce the issue, the expected outcome, and the actual result. If possible, include a minimal code example that demonstrates the problem.

## Using `cargo-rdme`

This project uses `cargo-rdme` to keep the README files in sync with the crate-level documentation. To ensure your changes are reflected in the README files, follow these steps:

1. Install `cargo-rdme` and `cargo-workspaces` using `cargo install cargo-rdme cargo-workspaces`.
2. Run `cargo workspaces exec -- cargo rdme` to update the README files based on the latest crate-level documentation.
3. Include the updated README files in your pull request.

Thank you for contributing to Litehouse!

