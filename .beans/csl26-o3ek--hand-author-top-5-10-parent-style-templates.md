---
# csl26-o3ek
title: Hand-author top 5-10 parent style templates
status: todo
type: task
priority: high
created_at: 2026-02-08T00:39:09Z
updated_at: 2026-02-08T00:39:09Z
blocking:
    - csl26-m3lb
---

Hand-author CSLN bibliography templates for the highest-impact parent styles, using examples/apa-style.yaml as the gold standard model and style guides as primary source.

Target styles (by dependent count):
1. apa (783 dependents) - DONE (examples/apa-style.yaml)
2. elsevier-with-titles (672)
3. elsevier-harvard (665)
4. springer-basic-author-date (460)
5. ieee (176)
6. elsevier-vancouver (163)
7. american-medical-association (114)
8. nature (76)
9. cell (57)
10. chicago-author-date (55)

Approach:
- Read the style guide or representative output for each style
- Author the bibliography template section as CSLN YAML
- Verify against oracle comparison (node scripts/oracle.js)
- Each style produces ~10-15 template components

These 10 styles cover ~60% of dependent styles. Combined with the working XML options pipeline, this should achieve high bibliography match rates.

~5-10 hours of domain-expert time.