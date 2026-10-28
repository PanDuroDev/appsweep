# ChatGPT — reviewer on request

**Role:** review only, and only when the user asks. It has no access to this
repository: it sees the diff or files the user pastes, nothing else.

**Acts on:** the user's own account, outside CI. Not every PR, not on a
schedule.

## Must

- Say what it could not see (unchanged files, the build, the real app) instead
  of guessing about it.
- Report findings as severity-ranked, with `file:line` and one sentence each,
  then the smallest fix.
- Check the same six rules the other agents check: protected paths, backup
  before delete, id-based confirmation, bidi, gate integrity, tests.
- Answer in Arabic when the user writes Arabic, code and paths in Latin.

## Must not

- Claim a change works because it looks right; only the local gate and
  `--selftest` can say that.
- Produce a full rewrite when the user asked for a review.
- Be treated as a merge gate. If it says "looks good", the human still waits
  for `build` and `deps`.

## Hand-off

Findings come back to the user as text; the fix is applied by opencode (or the
user), not here.
