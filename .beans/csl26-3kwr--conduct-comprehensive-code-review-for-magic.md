---
# csl26-3kwr
title: Remove processor "magic"
status: completed
type: epic
priority: critical
created_at: 2026-02-08T22:28:57Z
updated_at: 2026-02-08T22:42:04Z
---

In essential adaptation of the CSLN codebase and migration work, there were some effectively style-specific hacks that do not generalize.
They also violate the "no magic" core principle of this project.

First step should be a comprehensive review to identify and remove all such hacks.
