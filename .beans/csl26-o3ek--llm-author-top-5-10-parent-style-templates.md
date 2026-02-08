---
# csl26-o3ek
title: LLM-author top 5-10 parent style templates
status: todo
type: task
priority: high
created_at: 2026-02-08T00:39:09Z
updated_at: 2026-02-08T12:00:00Z
blocking:
    - csl26-m3lb
---

Create CSLN templates for the highest-impact parent styles using the `/styleauthor` skill or `@styleauthor` agent. The APA 7th Edition was the first style created this way (examples/apa-7th.yaml) and achieved 5/5 citation + 5/5 bibliography match.

Target styles (by dependent count):

1. apa (783 dependents) - DONE (examples/apa-7th.yaml)
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

Use the formalized `/styleauthor` workflow (see .claude/skills/styleauthor/SKILL.md):

1. Research: read style guide references and oracle output
2. Author: create CSLN YAML using examples/apa-7th.yaml as model
3. Test: run processor and compare output to expectations
4. Evolve: add missing processor features if needed (with cargo test guard)
5. Verify: oracle comparison + regression check

Invoke via:
- `/styleauthor <style-name> --urls <guide-url>` for interactive authoring
- `@styleauthor` agent for autonomous batch work

These 10 styles cover ~60% of dependent styles. Combined with the working XML options pipeline, this should achieve high bibliography match rates.

**Key insight:** LLM-authored styles are higher quality than migration-compiled styles because the LLM understands the style guide's intent, not just the CSL 1.0 XML structure. The LLM can also evolve processor code when features are missing, making this a full-stack workflow.

Reusable patterns are captured in .claude/skills/styleauthor/templates/common-patterns.yaml.