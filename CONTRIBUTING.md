# Contributing to Relay

Thank you for contributing to Relay! Please check existing issues and pull requests before creating new ones.

## Setup

Run `bun dev` from the app package to start the local Relay server.

Prerequisites:

- [Rust](https://rustup.rs)
- [Bun](https://bun.sh)
- [Node.js ≥ 22](https://nodejs.org)

```bash
cd app
bun install
bun dev
```

## Before Submitting

1. Code passes checks: `cd app && bun check`
2. Auto-fix issues: `cd app && bun fix`
3. Formatting only when needed: `cd app && bun format`
4. App runs: `cd app && bun dev`
5. Rebase on master: `git rebase origin/master`
6. Squash commits into logical units
7. Review and agree to the
   [Contributor License and Feedback Agreement](CONTRIBUTOR_LICENSE_AND_FEEDBACK_AGREEMENT.md)

## Guidelines

- Follow the existing code style
- Use descriptive commit messages (i.e., "Add autocompletion")
- One logical change per commit
- Update documentation if needed
