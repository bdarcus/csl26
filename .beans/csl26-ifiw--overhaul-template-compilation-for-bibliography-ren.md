---
# csl26-ifiw
title: Overhaul template compilation for bibliography rendering
status: todo
type: epic
priority: high
created_at: 2026-02-07T18:20:28Z
updated_at: 2026-02-07T18:20:28Z
---

Epic to track the template compilation improvements needed for bibliography rendering. The delimiter infrastructure (Tasks #1-6) is working, but migration has deeper issues in component selection, ordering, and suppress logic. Oracle verification shows 0/10 top parent styles with perfect bibliography match despite 60% citation match. This epic tracks the systematic fixes needed in csln_migrate/src/template_compiler/.