---
# csl26-kjzk
title: Consider boltffi for bindings
status: draft
type: task
priority: normal
created_at: 2026-02-17T22:02:32Z
updated_at: 2026-02-17T22:22:37Z
---

BoltFFI seems like it might be an interesting solution for this project:

https://github.com/boltffi/boltffi

It currently has "full support" for Swift, Kotlin, and Typescript/WASM, with plans
for Python, C#, and Ruby.

For languages that support it, it will also autogen async support on the other side, which should be valuable for our purposes. From the docs:

> Rust async functions can be exported just like regular functions.

I just submitted an issue for Lua support:

https://github.com/boltffi/boltffi/issues/49

I am thinking it might make for a more integrated but also targeted approach than separate FFI + WASM?
