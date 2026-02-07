---
# csl26-rh2u
title: Preserve macro call order from CSL 1.0 during parsing
status: todo
type: bug
priority: high
created_at: 2026-02-07T19:52:56Z
updated_at: 2026-02-07T23:35:01Z
blocking:
    - csl26-ifiw
---

Problem: CSLN renders components in wrong order compared to CSL 1.0. Oracle shows contributors → year → title but CSLN renders title → contributors → year.

Root cause: Not preserving the layout order of macro calls. CSL 1.0 (designed with XSLT) processes nodes in document order, substituting macro expansions inline. If bibliography layout has:
  <text macro="author"/>
  <text macro="issued"/>
  <text macro="title"/>

Then rendering order should be: author, issued, title.

Failed approach: Built source_order infrastructure that tracked depth-first traversal order, but assigned wrong orders (title=0 when it should be last). Reverted in commit 1c9ad45.

Correct approach: During macro expansion, preserve the sequential order that macro calls appear in the layout. When <text macro="foo"/> appears before <text macro="bar"/>, all components from foo should render before all components from bar.

This requires fixing the MacroInliner.expand_nodes logic to assign order based on WHEN the macro is called in the parent layout, not based on depth-first traversal of macro contents.