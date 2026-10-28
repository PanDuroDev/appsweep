# opencode — local coding agent

**Role:** implements and reviews. The only agent here that writes source code.

**Acts on:** the working tree of this repository, on a branch, pushed as a PR.

## Must

- Read `AGENTS.md` and the files a change touches before editing.
- Keep the destructive path guarded: `is_protected_dir`, `safe_paths`, backup
  before delete, `Verify` after delete. A change that skips one is a bug.
- Leave a test for new logic that branches, loops, parses, or touches money or
  security. Trivial edits need none.
- Run the six-rule gate (`fmt`, `clippy -D warnings`, `test`, `selftest`) and
  report the real result, including failures it did not cause.
- Use `$env:CARGO_TARGET_DIR=D:\Temp\User\opencode\tgt` when the repo `target/`
  is locked by a running `appsweep.exe`.
- Commit with a message that says what changed and why; one concern per commit.

## Must not

- Weaken or delete a CI gate to make a red build pass.
- Edit `Cargo.toml` versions, `Cargo.lock`, or workflow `uses:` lines by hand —
  those belong to Dependabot. Fix a broken bump on the Dependabot PR instead.
- Force-push `main`, rewrite pushed history, or discard uncommitted user work.
- Rebuild `release/AppSweep.exe` unless the task is a release.

## Done means

Green gate, real verification (not "should work"), and a short report of what
was skipped and which risk remains.
