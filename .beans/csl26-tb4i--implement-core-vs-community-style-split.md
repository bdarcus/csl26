---
# csl26-tb4i
title: Implement Core vs. Community Style Split
status: todo
type: task
priority: normal
created_at: 2026-02-08T18:45:51Z
updated_at: 2026-02-08T18:45:51Z
---

Adopt a hybrid strategy for style management. Focus the main repository on ~10-20 core 'parent' styles (APA, Chicago, IEEE, etc.) to serve as an integration test suite. Move the remaining 2,000+ journal-specific styles to a separate community repository (e.g., csln-styles) managed as a git submodule. This preserves the tight development loop while preventing repository bloat.