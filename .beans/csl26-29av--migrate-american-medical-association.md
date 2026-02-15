---
# csl26-29av
title: 'Migrate: american-medical-association'
status: completed
type: feature
priority: normal
created_at: 2026-02-15T14:36:00Z
updated_at: 2026-02-15T15:01:07Z
---

Locale term support implementation complete

✅ Implemented RoleLabel configuration system
✅ Added label field to TemplateContributor
✅ Processor renders labels from locale terms
✅ All tests passing (fmt, clippy, test)

Infrastructure:
- RoleLabel struct with term/form/placement
- RoleLabelForm enum (Short, Long)
- LabelPlacement enum (Prefix, Suffix)
- Integrated with existing locale.role_term()

AMA Status:
- Citations: 7/7 ✅
- Bibliography: Partial (editor labels working but needs refinement)
- Known issue: Duplicate rendering for edited books

Next steps:
1. Debug AMA duplicate editor rendering
2. Add locale override for lowercase "eds."
3. Test with other numeric styles (Vancouver, IEEE)
