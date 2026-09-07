---
# csl26-2hr4
title: Group tracker merges before checking if the group rendered
status: todo
type: bug
priority: high
tags:
    - engine
    - rendering
    - fidelity
created_at: 2026-09-06T23:13:42Z
updated_at: 2026-09-07T00:14:29Z
parent: csl26-8m2p
blocking:
    - csl26-8b4a
---

render_group_component_with_format (crates/citum-engine/src/processor/rendering/grouped/core.rs, ~line 1368) clones the tracker for a group's children, then unconditionally calls tracker.merge_from(group_tracker) BEFORE checking whether render_group_child_values actually produced output (the values? empty-check comes after the merge). So an empty/suppressed group's tracker mutations -- variable-once marks, substitution bookkeeping, date-fallback-first-issued flags -- still leak into the parent tracker even though the group rendered nothing.

Elevated (2026-09-06, second Codex review round): this is now a BLOCKING PREREQUISITE for alternatives: (csl26-8b4a), not an independent someday investigation. A second adversarial-review pass found that clone-and-discard at the alternatives: boundary alone is insufficient -- the merge-before-check happens at EVERY level of nesting inside render_group_component_with_format, so if the WINNING alternatives candidate is itself a group: containing a nested empty sub-group, that sub-group's tracker mutations are already baked into the candidate's own clone before alternatives: ever gets a say. Merging 'only the winner's' tracker still commits the pollution. alternatives: cannot be implemented with correct tracker isolation until this ordering is fixed (or explicitly proven safe as-is) -- see docs/specs/ALTERNATIVES.md's Implementation Notes and Acceptance Criteria for the dependency.

Impact on existing group: rendering is unclear -- may be intentional (once a variable is examined it should never be considered again regardless of group suppression) or may be a real bug (a suppressed group's exploratory tracker probing should not count). Needs investigation before deciding whether to fix.

## Todo
- [ ] Determine whether current merge-always behavior is intentional or a bug (check test coverage/git history for render_group_component_with_format's tracker handling)
- [ ] If a bug, fix by moving tracker.merge_from after the values? empty-check (or conditioning it on Some(_))
- [ ] Add a regression test: an empty/suppressed group followed by a component that should still be eligible to render the variable the suppressed group examined
- [ ] Add a regression test matching alternatives:'s forcing case: a winning alternatives candidate containing a suppressed/empty nested group, followed by a component that depends on tracker state the nested group must not have touched
