# Contributing to Tachyon ⚡

Thank you for your interest in contributing to Tachyon! We welcome contributions, bug reports, and performance optimizations from the community.

## How to Contribute

### 1. Reporting Issues
- Check the [existing issues](https://github.com/khuranajordan/tachyon/issues) to avoid duplicates.
- If your issue is new, open a new issue using our Bug Report or Feature Request templates.

### 2. Submitting Pull Requests
All changes to Tachyon are made via Pull Requests:
1. **Fork** the repository on GitHub: `https://github.com/khuranajordan/tachyon`.
2. **Clone** your fork locally:
   ```bash
   git clone https://github.com/<your-username>/tachyon.git
   cd tachyon
   ```
3. **Create a feature branch**:
   ```bash
   git checkout -b feat/your-feature-name
   ```
4. **Make your changes** and verify tests pass:
   ```bash
   cargo check
   cargo test
   ```
5. **Commit your changes**:
   ```bash
   git commit -m "feat: description of change"
   ```
6. **Push to your fork**:
   ```bash
   git push origin feat/your-feature-name
   ```
7. **Open a Pull Request** against `khuranajordan/tachyon:main`.

### Review & Merge Policy
- Only repository maintainers can merge Pull Requests into `main`.
- Direct pushes and force pushes to `main` are disabled.
- Maintainers review all PRs, run automated tests, and merge when approved.
