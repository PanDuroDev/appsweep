# CodeQL + GitHub Advanced Security — static analysis

**Role:** not an AI. Static security analysis of the Rust sources.

**Acts on:** push and pull request to `main`, plus a weekly cron (Mondays 03:17
UTC). `build-mode: none`, so it parses instead of compiling.

**Owns:** `.github/workflows/codeql.yml`. Default setup stays off — the two
cannot run at the same time, and the workflow is the one we can read.

## Where findings land

Security → Code scanning alerts. They never block a merge on their own; the
required checks are `build` and `deps`. A new alert is a real task: fix it, or
dismiss it with a reason, in the same week.

## Limits to know

- Rust support is in public preview, and `build-mode: none` does not see code
  generated during the build. Fewer results, not wrong ones.

## Rules

- Do not add `continue-on-error` or lower the query suite to silence a finding.
- Do not enable default setup "as well"; it disables this workflow.
