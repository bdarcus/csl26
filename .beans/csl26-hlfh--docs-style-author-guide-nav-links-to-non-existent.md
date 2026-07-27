---
# csl26-hlfh
title: 'docs: style-author-guide nav links to non-existent docs/news/index.html'
status: todo
type: bug
priority: low
created_at: 2026-07-27T22:20:10Z
updated_at: 2026-07-27T22:20:10Z
---

docs/guides/style-author-guide.template.html has two <a href="../news/index.html">News</a> nav links (desktop + mobile menu) that 404 -- docs/news/ does not exist anywhere in the repo history. Found incidentally while building a docs/**/*.html internal-link checker for csl26-6p3d, which special-cases this path to avoid a false CI failure. Either remove the nav item or build the news page/index.
