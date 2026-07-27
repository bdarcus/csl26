---
# csl26-8jtn
title: alint --base diff mode false-positives on unrelated broken-markdown-links
status: completed
type: bug
priority: normal
tags:
    - alint
    - hooks
    - tooling
created_at: 2026-07-27T13:08:05Z
updated_at: 2026-07-27T13:27:18Z
---

Not actually an alint bug -- root-caused and fixed in .githooks/pre-push.

Root cause: alint's own --help is explicit that `--base` implies `--changed`, which restricts which files count as existing for path-resolution rules to the diff's changed-file set (documented exemptions are the cross-file/existence rule families, e.g. `file_exists`; `markdown_paths_resolve` is not one of them). So a correct, untouched cross-reference in a touched doc gets reported as broken. Confirmed empirically in an isolated clone: touching only the referencing file reproduces the false positive; touching the referenced file too makes it disappear; reproduces identically on both alint 0.13.0 (pinned) and 0.14.0.

The actual bug was in citum-core's own .githooks/pre-push: it passed --base "$policy_base" as a CLI flag to alint purely to scope the conventional-commits rule, but that flag narrows every other rule's file-resolution scope too as a side effect. The conventional-commits rule doesn't need the CLI flag at all -- .alint.yml's rule already reads ALINT_BASE_SHA via its own since: template, independent of --changed/--base. This matches CI's own commit-policy.yml job, which sets only the ALINT_BASE_SHA env var and never passes --base -- which is exactly why CI never hit this.

## Summary of Changes

Removed the redundant --base "$policy_base" CLI flag from .githooks/pre-push's alint invocation, keeping only the ALINT_BASE_SHA env var. Verified in an isolated repro clone that: (1) the false positive on an untouched cross-referenced file disappears, and (2) the conventional-commits rule still correctly rejects an out-of-allowlist scope for a commit in the pushed range. No alint upstream changes needed -- this was a citum-core hook design issue, not a tool defect.
