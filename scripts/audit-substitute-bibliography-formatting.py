#!/usr/bin/env python3
"""
Bibliography-context substitute-title formatting taxonomy for a single style.

Companion tool to `scripts/audit-substitute-formatting.py` (which surveys
citation-context <substitute> formatting across the legacy corpus). This
script instead takes a single style's `report-core.js` output plus its
reference fixture and classifies every author-less bibliography row into
one of the taxonomy classes used by
docs/specs/SUBSTITUTED_TITLE_BIBLIOGRAPHY_FORMATTING.md:

  - quote-gap:      oracle wraps the promoted title in quotes, Citum doesn't
  - emphasis-gap:   oracle italicizes the promoted title, Citum doesn't
  - candidate-gap:  oracle promotes a DIFFERENT value (usually container-title)
                     than Citum does -- a candidate-selection defect, not a
                     formatting defect
  - render-when-bypass: the reference's type routes through a template branch
                     that never reaches `contributor: author` at all (e.g.
                     `webpage`'s title-absent branch) -- structurally outside
                     this spec's scope regardless of formatting outcome
  - match:          oracle and Citum already agree
  - unclassified:   doesn't fit any of the above; needs manual review

Usage:
    python3 scripts/audit-substitute-bibliography-formatting.py \\
        --report /tmp/report.json \\
        --fixture tests/fixtures/test-items-library/chicago-18th.json \\
        [--bypass-types webpage,interview] [--json]

Pass `--simulate` to additionally hand-simulate the spec's recommended fix
(derive a substituted title's formatting from the reference type's own
resolved `title: primary` node, category as base with the node's own
`Rendering` merged over it -- the same precedence
`effective_title_quote_depth` already uses for normal title rendering) for
every quote-gap/emphasis-gap row, and report whether that alone would close
each row. Requires `NODE_FORMAT` below to be hand-updated whenever the
target style's bibliography templates change -- see its own docstring.

Where `--report` is `node scripts/report-core.js --all-features --style
<style> > report.json` output and `--fixture` is the reference library the
style's oracle run used.

Known limits (read before trusting a row's classification):
  - "Which value each side promotes" is inferred by checking whether the
    reference's `title`/`container-title` text falls within a fixed-size
    LEAD_WINDOW of normalized characters at the start of each rendered
    string, ignoring markup. This has no single correct window size: too
    small and a long prefix (e.g. a corporate author rendered as publisher)
    pushes a genuinely-promoted title out of the window (false
    "unclassified"); too large and a short entry's container-title and
    title both land inside the window even when only the container was
    actually promoted (false "quote-gap"/"emphasis-gap" instead of the
    correct "candidate-gap"). At LEAD_WINDOW=60, four `article-magazine`/
    `article-newspaper` rows (`6188419/Y7JIURAM`, `6188419/L4XXFEU2`,
    `6188419/6V4XJV4M`, `6188419/MAWJL9U8`) are manually confirmed
    candidate-gap (oracle promotes `container-title`, Citum promotes
    `title`) but this script buckets them as quote-gap. Spot-check every
    row before citing it as evidence -- this is corroborating evidence for
    the taxonomy in
    `docs/specs/SUBSTITUTED_TITLE_BIBLIOGRAPHY_FORMATTING.md`, not a source
    of truth on its own, matching the caveat
    `scripts/audit-substitute-formatting.py` already states for its own
    classifier.
  - `--bypass-types` must be supplied by hand from reading the style's own
    bibliography template for `render-when: field-absent/field-present:
    title` branches that omit `contributor: author` entirely (see
    `chicago-author-date-18th.yaml`'s `webpage` entry for the canonical
    example). This script does not parse the style YAML's template
    structure; it only trusts the type list you give it.
  - Only bibliography entries are considered. Citation-context substitute
    formatting is `SUBSTITUTED_VALUE_FORMATTING.md`'s scope, not this one.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from collections import Counter
from pathlib import Path

SUBST_ROLES = ("author", "editor", "translator", "collection-editor", "collection_editor")

DEFAULT_BYPASS_TYPES = {"webpage"}

# Manually verified (not parsed) from chicago-author-date-18th.yaml's own
# bibliography templates at commit 5bb32707: what does each ref type's own
# resolved `title: primary` node declare, right now, on this branch? Stale
# the moment that YAML changes -- re-derive by hand, don't trust this table
# blindly. "default" means the type has no dedicated type-variant and falls
# through to the style's base/default template.
NODE_FORMAT = {
    "document": "default",       # no dedicated type-variant (yaml ~1263)
    "manuscript": "quote",       # "manuscript, collection:" (yaml ~658)
    "article-journal": "quote",  # yaml ~265
    "map": "italic",             # "map, graphic, classic, hearing:" (yaml ~930)
    "graphic": "italic",
    "classic": "italic",
    "hearing": "italic",
    "software": "plain",         # yaml ~971 -- text-case only, no wrap/emph
    "song": "plain",             # yaml ~1055 -- bare, no wrap/emph
    "speech": "default",         # no dedicated type-variant -> default template
}

# Four rows LEAD_WINDOW misclassifies as quote-gap when they're really
# candidate-gap (see the LEAD_WINDOW docstring below) -- excluded from
# --simulate's target set the same way the taxonomy table hand-corrects them.
KNOWN_CANDIDATE_GAP = {
    "6188419/Y7JIURAM",
    "6188419/L4XXFEU2",
    "6188419/6V4XJV4M",
    "6188419/MAWJL9U8",
}


def clean_div(raw: str | None) -> str:
    if not raw:
        return ""
    raw = re.sub(r'^\s*<div class="csl-entry">', "", raw)
    return re.sub(r"</div>\s*$", "", raw).strip()


def norm(text: str | None) -> str:
    return re.sub(r"[^a-z0-9]+", "", (text or "").lower())


def title_text(ref: dict) -> str | None:
    t = ref.get("title")
    if isinstance(t, dict):
        return t.get("main")
    return t if isinstance(t, str) else None


LEAD_WINDOW = 60  # normalized chars from the start of the plain-text output


def leads(raw: str, needle: str) -> bool:
    """Does `needle`'s text appear within the LEADING run of `raw` (markup stripped)?

    This is the check that distinguishes "this value was promoted into the
    author slot" from "this value merely appears somewhere later in the
    entry" (e.g. an article title that still renders in its own normal
    `title: primary` node after a *different* value -- usually
    `container-title` -- was promoted instead). Positional, not just
    presence -- a naive substring-anywhere check misclassifies exactly the
    candidate-gap rows this taxonomy exists to catch.
    """
    plain = re.sub(r"<[^>]+>|[_*]", "", clean_div(raw))
    n = norm(needle)[:20]
    return bool(n) and n in norm(plain)[:LEAD_WINDOW]


def wrapper_around(raw: str, needle: str, oracle: bool) -> str | None:
    """Classify how `needle`'s text is formatted inside `raw`, or None if absent."""
    text = clean_div(raw)
    n = norm(needle)[:20]
    if not n:
        return None
    plain = re.sub(r"<[^>]+>|[_*]", "", text)
    if n not in norm(plain):
        return "absent"
    spans = re.finditer(r"<i>(.*?)</i>", text) if oracle else re.finditer(r"_([^_]+)_", text)
    for m in spans:
        if n in norm(m.group(1)):
            return "italic"
    for m in re.finditer(r"[“\"]([^”\"]*)[”\"]", text):
        if n in norm(m.group(1)):
            return "quote"
    return "plain"


def is_substitute_eligible(ref: dict) -> bool:
    return not any(ref.get(k) for k in SUBST_ROLES)


def classify(entry: dict, ref: dict, bypass_types: set[str]) -> tuple[str, dict]:
    detail: dict = {}
    ref_type = ref.get("type")
    if ref_type in bypass_types:
        return "render-when-bypass", detail

    title = title_text(ref)
    container = ref.get("container-title")
    o_raw, c_raw = entry.get("rawOracle"), entry.get("rawCitum")

    o_leads_title = bool(title) and leads(o_raw, title)
    c_leads_title = bool(title) and leads(c_raw, title)
    o_leads_container = bool(container) and leads(o_raw, container)
    c_leads_container = bool(container) and leads(c_raw, container)

    if title and o_leads_title and c_leads_title:
        o_fmt = wrapper_around(o_raw, title, True)
        c_fmt = wrapper_around(c_raw, title, False)
    elif container and not title and o_leads_container and c_leads_container:
        o_fmt = wrapper_around(o_raw, container, True)
        c_fmt = wrapper_around(c_raw, container, False)
    else:
        detail["o_leads_title"] = bool(o_leads_title)
        detail["c_leads_title"] = bool(c_leads_title)
        detail["o_leads_container"] = bool(o_leads_container)
        detail["c_leads_container"] = bool(c_leads_container)
        if title and container and (o_leads_container != c_leads_container or o_leads_title != c_leads_title):
            return "candidate-gap", detail
        return "unclassified", detail

    detail["oracle_format"] = o_fmt
    detail["citum_format"] = c_fmt
    if o_fmt == c_fmt:
        return "match", detail
    if o_fmt == "quote":
        return "quote-gap", detail
    if o_fmt == "italic":
        return "emphasis-gap", detail
    return "unclassified", detail


def simulate_prediction(rawcitum: str, title: str, node_fmt: str) -> tuple[str, bool]:
    """Hand-apply node_fmt's formatting to the (currently plain) promoted
    title inside rawcitum. Case-insensitive match because Citum applies
    text-case transforms (e.g. title-case) before rendering, so the
    fixture's raw title rarely appears with its original casing."""
    text = clean_div(rawcitum)
    m = re.search(re.escape(title), text, re.IGNORECASE)
    if not m:
        return text, False
    idx, end = m.start(), m.end()
    rendered_title = text[idx:end]
    before, after = text[:idx], text[end:]
    if node_fmt in ("quote", "default"):
        new_title = f"“{rendered_title}”"
    elif node_fmt == "italic":
        new_title = f"_{rendered_title}_"
    else:
        new_title = rendered_title
    return before + new_title + after, True


def simulate(rows: list[dict], style: dict, by_id: dict) -> list[dict]:
    """For each quote-gap/emphasis-gap row, predict the mechanism's output
    and classify: clean-fix (matches oracle), content-gap-remains (right
    formatting, but the row still diverges for an unrelated reason), or
    no-effect (the node has no formatting to read, so nothing changes)."""
    entries_by_id = {e["id"]: e for e in style["bibliography"]["entries"]}
    out = []
    for r in rows:
        if r["class"] not in ("quote-gap", "emphasis-gap") or r["id"] in KNOWN_CANDIDATE_GAP:
            continue
        ref = by_id[r["id"]]
        entry = entries_by_id[r["id"]]
        title = title_text(ref)
        node_fmt = NODE_FORMAT.get(r["type"], "unknown")
        if not title:
            out.append({**r, "node_format": node_fmt, "sim_category": "unclassified"})
            continue
        predicted, found = simulate_prediction(entry.get("rawCitum"), title, node_fmt)
        oracle = clean_div(entry.get("rawOracle"))

        def loose(s: str) -> str:
            s = re.sub(r"<[^>]+>|[_*]", "", s)
            s = s.replace("“", "").replace("”", "").replace('"', "")
            return re.sub(r"[.\s]+", " ", s).strip().lower()

        content_complete = found and loose(predicted) == loose(oracle)
        if node_fmt == "plain":
            sim_category = "no-effect"
        elif content_complete:
            sim_category = "clean-fix"
        else:
            sim_category = "content-gap-remains"
        out.append(
            {
                **r,
                "node_format": node_fmt,
                "oracle": oracle,
                "citum_now": clean_div(entry.get("rawCitum")),
                "citum_predicted": predicted,
                "sim_category": sim_category,
            }
        )
    return out


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--report", required=True, type=Path)
    ap.add_argument("--fixture", required=True, type=Path)
    ap.add_argument("--bypass-types", default="", help="comma-separated ref types that bypass contributor:author")
    ap.add_argument("--json", action="store_true")
    ap.add_argument(
        "--simulate",
        action="store_true",
        help="hand-simulate the template-node mechanism for quote-gap/emphasis-gap rows",
    )
    args = ap.parse_args()

    report = json.loads(args.report.read_text())
    style = report["styles"][0]

    lib = json.loads(args.fixture.read_text())
    refs = lib if isinstance(lib, list) else lib.get("references", lib.get("items", []))
    by_id = {r.get("id"): r for r in refs}

    bypass_types = DEFAULT_BYPASS_TYPES | {t.strip() for t in args.bypass_types.split(",") if t.strip()}

    rows = []
    for entry in style["bibliography"]["entries"]:
        ref = by_id.get(entry["id"])
        if not ref or not is_substitute_eligible(ref):
            continue
        cls, detail = classify(entry, ref, bypass_types)
        rows.append({"id": entry["id"], "type": ref.get("type"), "class": cls, **detail})

    if args.simulate:
        sim_rows = simulate(rows, style, by_id)
        if args.json:
            print(json.dumps(sim_rows, indent=2, ensure_ascii=False))
            return 0
        print(f"style: {style['name']}")
        print(f"simulated rows (quote-gap + emphasis-gap, minus known candidate-gap): {len(sim_rows)}\n")
        by_type: dict[str, list[str]] = {}
        for r in sim_rows:
            by_type.setdefault(r["type"], []).append(r["sim_category"])
        print(f"{'type':16s} clean  content-gap  no-effect")
        for t, cats in sorted(by_type.items()):
            c = Counter(cats)
            print(f"  {t:16s} {c['clean-fix']:2d}     {c['content-gap-remains']:2d}           {c['no-effect']:2d}")
        overall = Counter(r["sim_category"] for r in sim_rows)
        print(
            f"\nTOTAL {len(sim_rows)}: clean-fix={overall['clean-fix']} "
            f"content-gap-remains={overall['content-gap-remains']} no-effect={overall['no-effect']}"
        )
        return 0

    if args.json:
        print(json.dumps(rows, indent=2, ensure_ascii=False))
        return 0

    print(f"style: {style['name']}")
    print(f"author-less bibliography rows: {len(rows)}\n")
    counts = Counter(r["class"] for r in rows)
    for cls in ("quote-gap", "emphasis-gap", "candidate-gap", "render-when-bypass", "match", "unclassified"):
        print(f"  {cls:20s} {counts.get(cls, 0)}")

    for cls in ("quote-gap", "emphasis-gap", "candidate-gap", "unclassified"):
        subset = [r for r in rows if r["class"] == cls]
        if not subset:
            continue
        print(f"\n--- {cls} by type ---")
        for t, n in sorted(Counter(r["type"] for r in subset).items()):
            print(f"  {t:20s} {n}")

    return 0


if __name__ == "__main__":
    sys.exit(main())
