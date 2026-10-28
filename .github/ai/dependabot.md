# Dependabot — dependency updates only

**Role:** opens PRs that bump dependencies. No code review, no logic changes.

**Acts on:** weekly (Monday), plus whenever a security advisory needs a fix.

**Owns:** `.github/dependabot.yml`, the version lines in `Cargo.toml`, and the
`uses:` lines in `.github/workflows/*.yml`. Everything else is out of its scope.

## What it opens

- `cargo`: one PR per week for all minor + patch bumps (grouped), plus one PR
  each for majors.
- `github-actions`: one grouped PR for the action versions.
- Labels: `dependencies` + `rust` or `ci`. Limits: 3 and 2 open PRs.

## Its gate

Every PR it opens runs the full `ci` (`build`) and the `deps` dependency-review
job, and both are required to merge. Green `build` means the Windows build,
clippy, tests and `--selftest` all still work; green `deps` means no new
high/critical advisory.

## Rules

- A bump PR must touch only the manifest, the lock and workflow `uses:` lines.
  If it touches `src/`, it is not a bump: stop and read it.
- Minor + patch are safe to merge on green. A **major** needs a human to read
  the changelog: `eframe`/`egui`, `winreg` and `windows` majors change APIs and
  are expected to need a fix commit on the PR.
- Never widen `open-pull-requests-limit` to "catch up"; the limit keeps the
  security PRs from queueing behind noise.
- Do not hand-edit a version to silence a failing bump. Fix the bump itself.
