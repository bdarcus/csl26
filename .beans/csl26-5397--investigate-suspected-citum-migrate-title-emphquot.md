---
# csl26-5397
title: Investigate suspected citum-migrate title emph+quote artifact
status: todo
type: task
priority: low
created_at: 2026-08-02T18:16:31Z
updated_at: 2026-08-02T18:16:31Z
---

citum-analyze --config-presets title-concern candidates include ~68 styles (n=35 {component:{emph,quote,text-case:title}}, n=33 {component:{emph,quote}}) where the extractor sets both emph:true and quote:true on the same title category. This is either a rare real convention (double-marking article titles) or a citum-migrate extraction artifact. Investigate the source styles (e.g. catholic-biblical-association, chicago-notes-archive-place-first-no-url) and OptionsExtractor's title handling to determine which. Do not name a TitlePreset for this shape until resolved. Surfaced during csl26-4aml (config-presets analyzer audit).
