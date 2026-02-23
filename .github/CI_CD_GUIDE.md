# CI/CD Guide for mistral.rs

This document describes the comprehensive CI/CD workflows and local Git hooks for the mistral.rs project.

## Overview

The CI/CD system is fully integrated with the Makefile-based build system and ensures code quality through automated checks at multiple stages:

1. **Local Git Hooks** - Pre-commit and pre-push validation
2. **GitHub Actions** - Automated CI/CD on push/PR
3. **Scheduled Jobs** - Daily validation of MCP servers

## Local Git Hooks

### Installation

Install Git hooks using the provided PowerShell script:

```powershell
.\scripts\setup\install-git-hooks.ps1
```

This installs:
- **pre-commit**: Runs before each commit
- **pre-push**: Runs before pushing to remote
- **commit-msg**: Validates commit message format

### Pre-Commit Hook

**Runs on**: `git commit`

**Actions**:
1. Auto-format code using `make fmt`
2. Quick compilation check with `make check`
3. Auto-fix linting issues with `make lint-fix`
4. Stage formatted/fixed files

**Bypass** (not recommended):
```bash
git commit --no-verify
```

### Pre-Push Hook

**Runs on**: `git push`

**Actions**:
1. Run full test suite with `make test`
2. Run PowerShell tests (if available)
3. Check for uncommitted changes

**Bypass** (not recommended):
```bash
git push --no-verify
```

### Commit Message Hook

**Runs on**: `git commit`

**Validates**: Conventional Commits format

**Format**:
```
<type>(<scope>): <subject>
```

**Valid types**:
- `feat` - New feature
- `fix` - Bug fix
- `docs` - Documentation changes
- `style` - Code style changes
- `refactor` - Code refactoring
- `perf` - Performance improvements
- `test` - Test changes
- `chore` - Maintenance tasks
- `ci` - CI/CD changes
- `build` - Build system changes

**Examples**:
```
feat(core): add support for Qwen3 models
fix(server): resolve CUDA memory leak
docs(readme): update build instructions
ci(workflows): add MCP validation
```

## GitHub Actions Workflows

### 1. Rust CI/CD Pipeline (`rust-ci.yml`)

**Triggers**:
- Push to `main`, `master`, `develop`
- Pull requests to these branches
- Manual dispatch

**Jobs**:

#### quick-check (Ubuntu, ~5 min)
- Fast compilation check
- Uses sccache for caching
- First to fail if code doesn't compile

#### format-check (Ubuntu, ~2 min)
- Validates Rust formatting
- Checks Python formatting (if applicable)
- Ensures consistent code style

#### lint (Ubuntu, ~10 min)
- Runs clippy lints
- Enforces code quality standards
- Catches common mistakes

#### test (Ubuntu, macOS, ~20 min)
- Runs full test suite
- Matrix strategy for multi-platform
- Uploads test results as artifacts

#### build-release (Multi-platform, ~45 min)
- **Windows + CUDA**: Full CUDA build with all features
- **Linux + CPU**: Basic CPU build
- **macOS + Metal**: Metal-accelerated build
- Uploads binary artifacts (30-day retention)

#### security-audit (Ubuntu, ~5 min)
- Scans dependencies for vulnerabilities
- Uses `cargo-audit`
- Reports security issues

#### ci-complete (Meta job)
- Checks status of all jobs
- Reports overall pipeline status

### 2. MCP Validation (`mcp-validation.yml`)

**Triggers**:
- Push to `main`, `master` with MCP-related changes
- Pull requests with MCP changes
- Daily at 2am UTC (scheduled)
- Manual dispatch

**Jobs**:

#### validate-config (Ubuntu, ~3 min)
- Validates `MCP_CONFIG.json` syntax
- Uses MCP Inspector
- Displays configuration summary

#### test-servers-linux (Ubuntu, ~10 min)
- Tests MCP server package availability
- Validates Redis connectivity
- Generates test report

#### test-servers-windows (Windows, ~15 min)
- Tests MCP servers on Windows
- Simulates PowerShell environment
- Validates Redis on Windows

#### integration-test (Ubuntu, ~25 min)
- Builds mistralrs-server
- Tests MCP integration
- Verifies end-to-end functionality

### 3. PowerShell Test Suite (`powershell-tests.yml`)

**Triggers**:
- Push/PR with PowerShell script changes
- Push/PR with Makefile or Cargo changes
- Manual dispatch

**Jobs**:

#### validate-scripts (Windows, ~5 min)
- Runs PSScriptAnalyzer
- Checks script syntax
- Reports linting issues

#### run-tests (Windows, ~20 min)
- Builds mistralrs-server
- Runs comprehensive test suite
- Uploads test results

#### test-model-scripts (Windows, ~10 min)
- Validates download scripts
- Checks help documentation
- Ensures script quality

#### test-launcher-scripts (Windows, ~5 min)
- Validates launcher scripts
- Checks for standard patterns
- Ensures consistency

## Artifacts

### Build Artifacts (30-day retention)
- `windows-cuda-binary` - Windows CUDA build
- `linux-cpu-binary` - Linux CPU build
- `macos-metal-binary` - macOS Metal build

### Test Artifacts (7-day retention)
- `test-results-*` - Rust test results
- `mcp-test-report-*` - MCP validation reports
- `powershell-test-results` - PowerShell test results
- `mcp-config` - MCP configuration snapshot

## Caching Strategy

### Cargo Registry Cache
```yaml
~/.cargo/bin/
~/.cargo/registry/index/
~/.cargo/registry/cache/
~/.cargo/git/db/
```

**Key**: `${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}`

### sccache (Build Artifact Cache)
- Configured via `mozilla-actions/sccache-action`
- Caches compiled artifacts
- Significantly speeds up builds (2-5 min vs 30-45 min)

### Target Directory
- Cached per job
- Reduces redundant compilation

## Performance Optimization

### Build Times

**First build** (cold cache):
- Windows CUDA: ~45 minutes
- Linux CPU: ~25 minutes
- macOS Metal: ~30 minutes

**Subsequent builds** (hot cache):
- With sccache: ~5-10 minutes
- Without sccache: ~15-20 minutes

### Parallel Execution
- Multiple jobs run in parallel
- Matrix strategy for platform builds
- Fail-fast disabled (all jobs complete)

## Environment Variables

### Rust Configuration
```yaml
RUST_BACKTRACE: 1
CARGO_TERM_COLOR: always
RUST_MIN_STACK: 8388608  # Windows stack size
```

### sccache Configuration
```yaml
RUSTC_WRAPPER: sccache
SCCACHE_GHA_ENABLED: true
```

### CUDA Configuration (Windows)
```yaml
NVCC_CCBIN: <path-to-cl.exe>
CUDA_PATH: <cuda-toolkit-path>
```

## Troubleshooting

### Workflow Fails on Quick Check
**Cause**: Code doesn't compile

**Solution**:
```bash
make check  # Run locally to see errors
```

### Workflow Fails on Tests
**Cause**: Test failures

**Solution**:
```bash
make test  # Run tests locally
cargo test -p <package-name> -- --nocapture  # Debug specific test
```

### Windows CUDA Build Fails
**Cause**: CUDA environment not configured

**Solution**:
- Workflow uses `Jimver/cuda-toolkit@v0.2.19` action
- Automatically installs CUDA 12.6.0
- Configures NVCC_CCBIN via vswhere

### MCP Validation Fails
**Cause**: MCP server package unavailable or deprecated

**Solution**:
- Check MCP_CONFIG.json for version updates
- Verify package exists on npm
- Update server configurations

### PowerShell Tests Timeout
**Cause**: Long-running model loading tests

**Solution**:
- Workflows use `-QuickTest` flag
- Skips performance and model loading tests
- Focus on core functionality

## Best Practices

### Commit Workflow
1. Make code changes
2. Run `make check` to verify compilation
3. Run `make test` to verify tests pass
4. Commit (pre-commit hook auto-formats and lints)
5. Push (pre-push hook runs tests)

### Pull Request Workflow
1. Create feature branch
2. Make changes with frequent local testing
3. Push to branch (triggers CI)
4. Create PR when CI passes
5. Address any CI failures before merge

### Release Workflow
1. Ensure all CI jobs pass
2. Tag release version
3. Download binary artifacts from GitHub Actions
4. Attach to GitHub Release

## Manual Workflow Dispatch

All workflows support manual triggering via GitHub Actions UI:

1. Go to **Actions** tab
2. Select workflow
3. Click **Run workflow**
4. Select branch
5. Click **Run workflow** button

## Monitoring

### CI Status Badges

Add to README.md:

```markdown
[![Rust CI](https://github.com/EricLBuehler/mistral.rs/workflows/Rust%20CI%2FCD%20Pipeline/badge.svg)](https://github.com/EricLBuehler/mistral.rs/actions)
[![MCP Validation](https://github.com/EricLBuehler/mistral.rs/workflows/MCP%20Server%20Validation/badge.svg)](https://github.com/EricLBuehler/mistral.rs/actions)
[![PowerShell Tests](https://github.com/EricLBuehler/mistral.rs/workflows/PowerShell%20Test%20Suite/badge.svg)](https://github.com/EricLBuehler/mistral.rs/actions)
```

### Notifications

GitHub Actions notifications:
- Email on workflow failure (default)
- Slack/Discord webhook integration (optional)
- GitHub mobile app push notifications

## Security

### Secrets Management
- No secrets required for public builds
- Optional: `GITHUB_TOKEN` (auto-provided)
- For private: Add secrets in repo settings

### Dependency Auditing
- `cargo audit` runs on every workflow
- Checks for known vulnerabilities
- Updates Cargo.lock automatically

### Code Scanning
- CodeQL analysis (optional, can be added)
- Dependabot alerts (GitHub native)
- Security advisories monitoring

## Maintenance

### Updating Workflows
1. Edit workflow YAML files
2. Test with workflow dispatch
3. Commit changes
4. Monitor first run

### Updating Dependencies
```bash
cargo update
make test  # Verify nothing breaks
git commit -m "chore(deps): update Rust dependencies"
```

### Updating Git Hooks
1. Edit files in `.githooks/`
2. Run `.\scripts\setup\install-git-hooks.ps1`
3. Test with dummy commits

## FAQ

**Q: How do I skip CI checks?**
A: Don't. But if absolutely necessary: `[skip ci]` in commit message.

**Q: How do I retry failed jobs?**
A: Click "Re-run failed jobs" in GitHub Actions UI.

**Q: Can I run workflows locally?**
A: Yes, use [act](https://github.com/nektos/act) for local GitHub Actions.

**Q: How do I add a new job?**
A: Edit workflow YAML, add job definition, ensure dependencies are correct.

**Q: Why is my build taking so long?**
A: First build always takes longer. Subsequent builds use cache.

## Support

For issues with CI/CD:
1. Check workflow logs in GitHub Actions
2. Run `make ci` locally to reproduce
3. Open issue with workflow run URL
4. Tag with `ci/cd` label

---

**Last Updated**: 2025-10-03
**Version**: 1.0.0
**Maintainer**: mistral.rs CI/CD Team
