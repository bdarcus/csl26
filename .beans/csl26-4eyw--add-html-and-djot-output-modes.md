---
# csl26-4eyw
title: Add HTML and Djot output modes
status: completed
type: feature
priority: high
created_at: 2026-02-07T11:47:18Z
updated_at: 2026-02-07T12:12:29Z
parent: csl26-ismq
---

Ideally they should both be creating clean, semantically-enhanced, output; for example, adding semantic classes, perhaps like:

```html
<i class="csln-title">Some Title</i>
```

In [Djot](https://github.com/jgm/djot/tree/main):

```djot
[Some Title]{.csln-title}
```
