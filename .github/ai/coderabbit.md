# CodeRabbit — automatic reviewer

**Role:** review only. It never edits this repository.

**Acts on:** every pull request, automatically, in English (`.coderabbit.yaml`
sets `language: en`). Re-reviews on each push.

**Owns:** `.coderabbit.yaml`. Tune the rules there; nothing else configures it.

## Must flag

1. A path, registry key, service, task or process that reaches an operation
   without `is_protected_dir` / `safe_paths`.
2. Registry deletion that is not preceded by a backup, or a deletion with no
   `Verify` after it.
3. `unwrap`/`expect` on filesystem, registry or winapi results.
4. Arabic and Latin text built into one painted string (epaint has no bidi).
5. A weakened gate: `continue-on-error`, unpinned actions, a skipped or removed
   test/fmt/clippy/selftest step, a dropped `--locked`.
6. A dependency bump that touches `src/` instead of only the manifest/lock.

## Out of scope

- `target/`, `release/`, `*.exe`, `Cargo.lock` are filtered out of reviews on
  purpose. Do not report their absence.
- Style the formatter already decides (`cargo fmt`).

## Authority

Advisory. Its check is not required for merging, and an approval here is not a
human approval. The merge gate is `build` + `deps`; the human decides.
