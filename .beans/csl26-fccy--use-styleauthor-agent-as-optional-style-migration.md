---
# csl26-fccy
title: Use styleauthor agent as optional style migration path
status: todo
type: feature
priority: high
created_at: 2026-02-08T20:59:06Z
updated_at: 2026-02-08T21:04:41Z
parent: csl26-m3lb
---

# An idea to smooth 1.0 style migration

This might be enhancement to the migration tool and/or the styleauthor skill.

Instead of using the template inferrer, this would allow use of the styleauthor agent to write the style itself, and iterate until it matches the output of the citeproc-js.

We could initially just use it for the top 10 or so styles, which we would maintain here.


# Possible example prompt

Use @styleauthor to take the top 10 CSL 1.0 styles and "hand author" CSLN equivalents. 

The target output formatting is whatever citeproc-js produces. 

Draw the metadata from the original style.

Spawn subagents as sensible, including @researcher to collect missing information, examples, or input data.
