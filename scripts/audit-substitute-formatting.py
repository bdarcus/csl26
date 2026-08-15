#!/usr/bin/env python3
"""
Substitute-formatting audit for citum-core.

Surveys the legacy CSL corpus (styles-legacy/*.csl) for how a value promoted
into the author slot via <substitute> is formatted, split by substituted-value
kind:

  - title (and parent-serial-title): does the style italicize, quote, both
    (conditionally on reference type), or leave it plain?
  - contributor roles (<names variable="editor"/> etc.): does the style leave
    the <names> element empty (inherit the slot's own name-list formatting)
    or override it with explicit <name>/<label>/<et-al> children (keep the
    substituted role's own formatting)?

This is corroborating evidence for docs/specs/SUBSTITUTED_VALUE_FORMATTING.md
-- not a source of truth on its own. See that spec's "Corpus method and
limits" section for what this classifier does and does not capture:
  - only the FIRST title-bearing element found inside a <substitute> block is
    classified; later fallback tiers in the same chain are not distinguished
  - macro calls are expanded up to 4 levels deep to find <text variable="title"
    .../> or a directly-styled bare variable
  - "italic-or-quote by reference type" is inferred from the co-presence of
    font-style="italic", quotes="true", and a <choose> in the expanded macro
    body -- it does not verify which types get which treatment
  - <names> child overrides are not further classified by what they do
    (label text, name form, et-al truncation) -- only that an override exists
  - the container-title (parent-serial) classifier below is intentionally
    narrower than the title classifier: it only recognizes a bare
    <text variable="container-title" .../> directly inside a <substitute>
    block, plus macro calls whose name contains "container" -- it does NOT
    expand arbitrary macro closures the way the title classifier does, so it
    under-counts relative to the title numbers and should be read as a lower
    bound, not a comparable percentage. This is corpus corroboration for the
    div-011 "parent-serial substitutes are never quoted" engine-mechanism
    claim, not a claim that this classifier is as thorough as the title one.

Corpus revision: numbers below were computed against the `styles-legacy`
submodule pinned at commit `ca545f945a676a4b6319ba386ef3adaccacf9df9`
(2844 `.csl` files, matching `origin/main` at the time this script was
written). Run `git -C styles-legacy log -1 --format='%H'` and compare
before treating a different count as a bug in this script -- upstream
Zotero styles churn, so the corpus is a moving target.

Usage:
    python3 scripts/audit-substitute-formatting.py [--json]
    python3 scripts/audit-substitute-formatting.py --style apa,chicago-author-date
"""

import argparse
import collections
import glob
import json
import re
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
STYLES_LEGACY = REPO_ROOT / "styles-legacy"

MACRO_RE = re.compile(r'<macro name="([^"]+)">(.*?)</macro>', re.S)
CITATION_RE = re.compile(r"<citation[ >].*?</citation>", re.S)
SUBSTITUTE_RE = re.compile(r"<substitute>(.*?)</substitute>", re.S)
MACRO_CALL_RE = re.compile(r'<text macro="([^"]+)"')
TITLE_VAR_RE = re.compile(r'<text variable="(title[^"]*)"([^/>]*)')
CONTAINER_TITLE_VAR_RE = re.compile(r'<text variable="(container-title)"([^/>]*)')
CONTAINER_MACRO_CALL_RE = re.compile(r'<text macro="([^"]*container[^"]*)"')
NAMES_RE = re.compile(r'<names variable="([^"]+)"\s*(/?)>(.*?)(?:</names>|\Z)', re.S)

MAX_MACRO_DEPTH = 4


def macro_closure(seed_names, macros):
    """All macro names transitively reachable from seed_names, in breadth-first
    discovery order (nearest calls first).

    Returns a list, not a set: callers use this order to decide which
    reachable <substitute> block to classify first when several are found,
    so the return value must be deterministic. An earlier version returned a
    bare set() here -- CPython randomizes str hash values per process by
    default, so set iteration order (and therefore classification results
    for styles whose closure reaches more than one substitute block with
    different formatting, e.g. apa.csl's long-form and short-form title
    macros) varied between runs of the identical script against the
    identical corpus. Confirmed by running unchanged code three times in a
    row and getting two different citation-context title tallies.
    """
    seen = set()
    order = []
    queue = collections.deque(seed_names)
    while queue:
        name = queue.popleft()
        if name in seen or name not in macros:
            continue
        seen.add(name)
        order.append(name)
        queue.extend(MACRO_CALL_RE.findall(macros[name]))
    return order


def expand_macro(body, macros, depth=0, seen=None):
    """Inline macro bodies referenced from `body`, up to MAX_MACRO_DEPTH."""
    if depth > MAX_MACRO_DEPTH:
        return body
    seen = seen or set()
    out = body
    for name in MACRO_CALL_RE.findall(body):
        if name in macros and name not in seen:
            out += expand_macro(macros[name], macros, depth + 1, seen | {name})
    return out


def classify_title_formatting(bodies, macros):
    """Classify the first title-bearing element found across `bodies`.

    Returns one of: 'italic', 'italic-or-quote-by-type', 'quote', 'plain',
    or None if no title element was found in any <substitute> block.
    """
    for body in bodies:
        for sub in SUBSTITUTE_RE.findall(body):
            title_macros = [m for m in MACRO_CALL_RE.findall(sub) if "title" in m]
            bare_var = TITLE_VAR_RE.findall(sub)
            if title_macros:
                expanded = "".join(expand_macro(macros.get(m, ""), macros) for m in title_macros)
                italic = 'font-style="italic"' in expanded
                quote = 'quotes="true"' in expanded
                conditional = "<choose" in expanded
                if italic and quote:
                    return "italic-or-quote-by-type" if conditional else "italic-and-quote-flat"
                if italic:
                    return "italic"
                if quote:
                    return "quote"
                return "plain"
            if bare_var:
                attrs = bare_var[0][1]
                if 'font-style="italic"' in attrs:
                    return "italic"
                if 'quotes="true"' in attrs:
                    return "quote"
                return "plain"
    return None


def audit_titles(files):
    """Classify substitute-title formatting, both anywhere and citation-reachable."""
    macro_re_cache = {}
    all_stats = collections.Counter()
    cit_stats = collections.Counter()
    examples = collections.defaultdict(list)
    per_style = {}

    for path in files:
        name = path.stem
        src = path.read_text(encoding="utf-8", errors="replace")
        macros = dict(MACRO_RE.findall(src))
        macro_re_cache[name] = macros

        any_kind = classify_title_formatting([src], macros)
        all_stats[any_kind or "no-title-in-substitute"] += 1

        citation = CITATION_RE.search(src)
        if citation:
            reach = macro_closure(MACRO_CALL_RE.findall(citation.group(0)), macros)
            bodies = [citation.group(0)] + [macros[m] for m in reach]
            cit_kind = classify_title_formatting(bodies, macros)
        else:
            cit_kind = None
        cit_stats[cit_kind or "no-title-in-substitute"] += 1

        per_style[name] = {"any": any_kind, "citation": cit_kind}
        if any_kind and len(examples[f"any:{any_kind}"]) < 5:
            examples[f"any:{any_kind}"].append(name)
        if cit_kind and len(examples[f"citation:{cit_kind}"]) < 5:
            examples[f"citation:{cit_kind}"].append(name)

    return {
        "any_context": dict(all_stats),
        "citation_context": dict(cit_stats),
        "examples": dict(examples),
        "per_style": per_style,
    }


def classify_container_title_formatting(bodies, macros):
    """Classify the first container-title (parent-serial) element found.

    Narrower than classify_title_formatting -- see the module docstring.
    Returns one of: 'italic', 'quote', 'plain', 'macro-container-call'
    (a container-related macro call was found but not expanded/classified),
    or None if no container-title element was found in any <substitute>.
    """
    for body in bodies:
        for sub in SUBSTITUTE_RE.findall(body):
            bare_var = CONTAINER_TITLE_VAR_RE.findall(sub)
            if bare_var:
                attrs = bare_var[0][1]
                if 'font-style="italic"' in attrs:
                    return "italic"
                if 'quotes="true"' in attrs:
                    return "quote"
                return "plain"
            container_macros = CONTAINER_MACRO_CALL_RE.findall(sub)
            if container_macros:
                return "macro-container-call"
    return None


def audit_container_titles(files):
    """Classify substitute container-title (parent-serial) formatting.

    See classify_container_title_formatting's docstring: this is a narrower,
    lower-bound classifier than audit_titles, not a directly comparable one.
    """
    all_stats = collections.Counter()
    cit_stats = collections.Counter()

    for path in files:
        src = path.read_text(encoding="utf-8", errors="replace")
        macros = dict(MACRO_RE.findall(src))

        any_kind = classify_container_title_formatting([src], macros)
        all_stats[any_kind or "no-container-title-in-substitute"] += 1

        citation = CITATION_RE.search(src)
        if citation:
            reach = macro_closure(MACRO_CALL_RE.findall(citation.group(0)), macros)
            bodies = [citation.group(0)] + [macros[m] for m in reach]
            cit_kind = classify_container_title_formatting(bodies, macros)
        else:
            cit_kind = None
        cit_stats[cit_kind or "no-container-title-in-substitute"] += 1

    return {"any_context": dict(all_stats), "citation_context": dict(cit_stats)}


def audit_contributors(files):
    """Classify <names> elements inside <substitute> as slot-inherited or overridden."""
    stats = collections.Counter()
    child_freq = collections.Counter()
    styles_with_override = set()

    for path in files:
        src = path.read_text(encoding="utf-8", errors="replace")
        for sub in SUBSTITUTE_RE.findall(src):
            for _var, self_closed, body in NAMES_RE.findall(sub):
                if self_closed == "/":
                    stats["slot-inherited (empty)"] += 1
                    continue
                first_segment = body.split("</names>")[0]
                children = set(re.findall(r"<(name|label|et-al|substitute)[ />]", first_segment))
                if not children:
                    stats["slot-inherited (empty)"] += 1
                else:
                    stats["source-overridden (explicit children)"] += 1
                    styles_with_override.add(path.stem)
                    for child in children:
                        child_freq[child] += 1

    return {
        "names_elements": dict(stats),
        "child_element_frequency": dict(child_freq),
        "styles_with_override": len(styles_with_override),
        "styles_total": len(files),
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--json", action="store_true", help="Output machine-readable JSON")
    parser.add_argument("--style", help="Comma-separated list of style basenames (no .csl) to audit instead of the full corpus")
    args = parser.parse_args()

    if args.style:
        files = [STYLES_LEGACY / f"{s.strip()}.csl" for s in args.style.split(",")]
        missing = [f for f in files if not f.exists()]
        if missing:
            print(f"error: not found: {', '.join(str(m) for m in missing)}", file=sys.stderr)
            sys.exit(1)
    else:
        files = sorted(STYLES_LEGACY.glob("*.csl"))

    titles = audit_titles(files)
    container_titles = audit_container_titles(files)
    contributors = audit_contributors(files)
    result = {
        "corpus_size": len(files),
        "titles": titles,
        "container_titles": container_titles,
        "contributors": contributors,
    }

    if args.json:
        print(json.dumps(result, indent=2, sort_keys=True))
        return

    print(f"Corpus: {len(files)} styles\n")
    print("=== Substituted title formatting (any context) ===")
    for kind, count in sorted(titles["any_context"].items(), key=lambda kv: -kv[1]):
        print(f"  {count:5d}  {kind}")
    print("\n=== Substituted title formatting (citation-reachable) ===")
    for kind, count in sorted(titles["citation_context"].items(), key=lambda kv: -kv[1]):
        print(f"  {count:5d}  {kind}")
    print("\n=== Substituted container-title (parent-serial) formatting (citation-reachable) ===")
    print("    (narrower classifier -- lower bound, not comparable to the title numbers above)")
    for kind, count in sorted(container_titles["citation_context"].items(), key=lambda kv: -kv[1]):
        print(f"  {count:5d}  {kind}")
    print("\n=== Substituted contributor <names> elements ===")
    for kind, count in sorted(contributors["names_elements"].items(), key=lambda kv: -kv[1]):
        print(f"  {count:5d}  {kind}")
    print(
        f"\nStyles with at least one overridden substitute <names>: "
        f"{contributors['styles_with_override']} / {contributors['styles_total']}"
    )
    print(f"Override child element frequency: {contributors['child_element_frequency']}")


if __name__ == "__main__":
    main()
