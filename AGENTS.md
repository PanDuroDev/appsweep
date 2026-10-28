# AGENTS.md

AppSweep is a Windows app manager and deep uninstaller: Rust + egui, plus one
C++ helper (`native/helper.cpp`), shipped as a single `.exe`. Windows 10 first.

This file is the contract for every agent that works in this repository,
human-directed or automatic. Each agent has its own file in `.github/ai/`.

## Six rules, no exceptions

1. **The destructive path is the product.** `src/actions.rs::build_plan` turns an
   app into steps. Every path, registry key, service, scheduled task and process
   must pass `model::is_protected_dir` / `actions::safe_paths`; the registry is
   backed up before `DeleteReg`; `Verify` follows every deletion.
2. **`AppEntry.id` identifies an app, never its name.** Names contain glyphs that
   cannot be typed, so confirmations and search key off the numeric id.
3. **epaint has no bidi.** Arabic and Latin text never go into one painted
   string; use `ui::widgets::bidi_runs` or separate widgets.
4. **Never weaken a gate.** No `continue-on-error`, no unpinned `@main`, no
   removed or skipped fmt/clippy/test/selftest step, `--locked` stays.
5. **Tests are pure logic** (no filesystem or registry writes); the destructive
   path is covered by `--selftest` in a temp dir. New non-trivial logic leaves a
   test or an assert-based self-check.
6. **Green before push:** `cargo fmt --check`, `cargo clippy --release --locked
   -- -D warnings`, `cargo test --release --locked`,
   `cargo run --release --locked -- --selftest`.

The product is English: every UI string, doc, commit message, issue and PR is
written in English, even when the user writes in another language.

## Who edits, who reviews

| Agent | Edits code | Reviews | Where it acts |
| --- | --- | --- | --- |
| opencode (local coding agent) | yes | yes | working tree, branches, PRs |
| CodeRabbit | no | yes, automatic | every PR |
| ChatGPT | no | yes, on request | outside the repo, the user's account |
| Dependabot | manifests and workflow `uses:` only | no | the PRs it opens |
| CodeQL + GitHub Advanced Security | no | static analysis, not AI | push/PR/weekly, Security tab |

A PR to `main` merges only when the `build` and `deps` checks are green. The
other checks are advisory on purpose; a human still decides.

## Agent files

- `.github/ai/opencode.md` — the coding agent: implements, fixes, verifies.
- `.github/ai/coderabbit.md` — automatic reviewer.
- `.github/ai/chatgpt.md` — reviewer on request, no repo access.
- `.github/ai/dependabot.md` — dependency updates only.
- `.github/ai/codeql.md` — static analysis and where its findings land.
