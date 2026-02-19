---
# csl26-o3ek
title: LLM-author top 5-10 parent style templates
status: in-progress
type: task
priority: high
created_at: 2026-02-08T00:39:09Z
updated_at: 2026-02-15T16:00:00Z
blocking:
    - csl26-m3lb
---

Create CSLN templates for the highest-impact parent styles using the `/styleauthor` skill or `@styleauthor` agent. The APA 7th Edition was the first style created this way (styles/apa-7th.yaml) and achieved 14/15 bibliography match.

Target styles (by dependent count):

1. apa (783 dependents) - ✅ CONVERTED (styles/apa-7th.yaml, 14/15 bib)
2. elsevier-with-titles (672) - ✅ CONVERTED (0/15 bib, year positioning)
3. elsevier-harvard (665) - ✅ CONVERTED (8/15 bib)
4. springer-basic-author-date (460) - ✅ CONVERTED (quality TBD)
5. ieee (176) - ✅ CONVERTED (15/15 cit, 0/15 bib, numeric blocker)
6. elsevier-vancouver (502) - ✅ CONVERTED (15/15 cit, 0/15 bib)
7. american-medical-association (293) - ✅ CONVERTED (7/7 cit, bib gaps)
8. chicago-author-date (234) - ✅ CONVERTED (6/15 bib)
9. taylor-and-francis-chicago-author-date (234) - ✅ CONVERTED
10. springer-vancouver-brackets (472) - ✅ CONVERTED

**Current Status (2026-02-19):**
- 10/10 styles converted to YAML
- Citations (strict oracle, 8 scenarios): 0/10 at 8/8
- Bibliography quality varies: 0-27/34 depending on style
- Production style directory now contains only schema-valid styles; drafts moved to `styles/experimental/`

**Next Steps:**

Phase 1: Author-Date Quality Refinement (4 styles, 40% corpus)
- APA: Maintain 27/27 bibliography while lifting citations from 7/8 to 8/8
- Elsevier Harvard: Raise citations (4/8) and bibliography (24/28)
- Taylor & Francis Chicago: Raise bibliography from 3/29
- Annals AAG: Raise bibliography from 5/28

Phase 2: Numeric Style Features (blockers for 6 styles, 20% corpus)
- Implement year positioning fix (all numeric styles at 0/15)
- Citation numbering and superscript support
- Then iterate numeric styles to 8/15+ bibliography

Phase 3: Workflow Optimization
- Document successful patterns from APA 14/15
- Identify common failure modes (year positioning, volume/issue)
- Optimize /styleauthor iteration budget (target: 18min/style)
- Create reusable templates in .claude/skills/styleauthor/templates/

**Workflow:**

Use the formalized `/styleauthor` workflow (see .claude/skills/styleauthor/SKILL.md):

1. Research: read style guide references and oracle output
2. Author: create CSLN YAML using styles/apa-7th.yaml as model
3. Test: run processor and compare output to expectations
4. Evolve: add missing processor features if needed (with cargo test guard)
5. Verify: oracle comparison + regression check

Invoke via:
- `/styleauthor <style-name> --urls <guide-url>` for interactive authoring
- `@styleauthor` agent for autonomous batch work

**Key insight:** LLM-authored styles are higher quality than migration-compiled styles because the LLM understands the style guide's intent, not just the CSL 1.0 XML structure. The LLM can also evolve processor code when features are missing, making this a full-stack workflow.

These 10 styles cover ~60% of dependent styles (4,792/7,987). Combined with the working XML options pipeline, this should achieve high bibliography match rates.

Refs: docs/TIER_STATUS.md, docs/architecture/ROADMAP.md
