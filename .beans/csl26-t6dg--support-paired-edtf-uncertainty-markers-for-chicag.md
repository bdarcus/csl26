---
# csl26-t6dg
title: Support paired EDTF uncertainty markers for Chicago guessed dates
status: todo
type: bug
priority: normal
tags:
    - dates
    - chicago
    - fidelity
created_at: 2026-07-28T12:18:12Z
updated_at: 2026-07-28T12:18:12Z
parent: csl26-h7oc
blocking:
    - csl26-giun
---

Logical follow-up to `csl26-giun`; parented to its owning Chicago feature because the bean schema does not allow a bug to have a task parent. This bug blocks `csl26-giun`.

Verified conversion evidence: CSL JSON `issued: 1750?` converts to native EDTF `issued: 1750?`; the conversion layer is truthful. Chicago's CSL oracle renders `Smith, John. [1750?]. Title of First Work.`, while Citum renders `Smith, John. 1750? Title of First Work.`. The compatibility report records this as a lenient match and exact-parity failure.

Acceptance criteria:
- [ ] Support paired uncertainty markers around an EDTF uncertain year, not only a suffix marker.
- [ ] Provide Chicago configuration that renders guessed dates as `[year?]`.
- [ ] Add schema, engine, and Chicago regression coverage for uncertain EDTF years.
- [ ] Regenerate committed schemas and data-model references required by the schema change.
- [ ] Preserve truthful CSL JSON `issued: 1750?` to native EDTF `1750?` conversion.
