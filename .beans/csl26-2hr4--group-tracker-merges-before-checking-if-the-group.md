---
# csl26-2hr4
title: Group tracker merges before checking if the group rendered
status: todo
type: bug
priority: normal
tags:
    - engine
    - rendering
    - fidelity
created_at: 2026-09-06T23:13:42Z
updated_at: 2026-09-06T23:13:47Z
parent: csl26-8m2p
---

render_group_component_with_format (crates/citum-engine/src/processor/rendering/grouped/core.rs, ~line 1368) clones the tracker for a group's children, then unconditionally calls tracker.merge_from(group_tracker) BEFORE checking whether render_group_child_values actually produced output (the values? empty-check comes after the merge). So an empty/suppressed group's tracker mutations -- variable-once marks, substitution bookkeeping, date-fallback-first-issued flags -- still leak into the parent tracker even though the group rendered nothing.

Found while verifying a Codex adversarial-review finding on docs/specs/ALTERNATIVES.md (csl26-8b4a): alternatives:'s own candidate evaluation must NOT inherit this behavior (a losing candidate's tracker mutations must be discarded, not merged), which is documented there. This bean is for the pre-existing group: behavior itself, which is unaffected by and out of scope for that spec.

Impact on existing group: rendering is unclear -- may be intentional (once a variable is examined it should never be considered again regardless of group suppression) or may be a real bug (a suppressed group's exploratory tracker probing should not count). Needs investigation before deciding whether to fix.

## Todo
- [ ] Determine whether current merge-always behavior is intentional or a bug (check test coverage/git history for render_group_component_with_format's tracker handling)
- [ ] If a bug, fix by moving tracker.merge_from after the values? empty-check (or conditioning it on Some(_))
- [ ] Add a regression test: an empty/suppressed group followed by a component that should still be eligible to render the variable the suppressed group examined
