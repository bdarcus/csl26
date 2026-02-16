The design decision “year suffix sequence restarts per group” is not generally correct for real-world academic practice; it accurately describes what some legal or highly sectionalized styles might want, but it is not the default expectation for mainstream scholarly author–date styles, so it should be configurable.[^1][^2][^3]

### How suffixes are understood in common academic styles

- Major author–date styles (e.g., APA and Harvard variants) define the a, b, c letters as part of the *global* identity of a work in a document: once something is “2020a”, it is “2020a” everywhere, regardless of section.[^2][^4][^3]
- These suffixes are tied to the reference-list ordering (alphabetically by title or similar) and are then reused consistently in all in‑text citations; guidance explicitly says “the letter appears in the in-text citation and the reference entry,” implying a single sequence per document, not per section.[^5][^3][^1][^2]
- Many institutional “Harvard” guides adopt the same pattern (add $a, b$ when same author and year; the ordered list and suffixes apply across the whole bibliography).[^6][^7]

So for most academic users working with a single bibliography or a conventional chaptered thesis, the intuitive expectation is that “Smith 2020a” in chapter 1 is still “Smith 2020a” in chapter 5, not that the letters restart in each group.[^6][^2]

### When per-group restarts *do* make sense

- In documents that have genuinely separate bibliographic sections—e.g., separate reference lists for each chapter or for clearly distinct parts of a book—some styles or house rules can and do treat each section as its own universe, so “2020a” is scoped to that section.[^8][^9]
- Legal practice adds another twist: legal citation often uses per-section numbering for footnotes, authorities, or tables (e.g., footnotes resetting per chapter, separate “Table of Authorities” sections), and analogous per-section disambiguation or numbering can be required by those conventions.[^10][^11]

Your rationale bullets are thus **sometimes** true, but not uniformly:

- “Grouped bibliographies are conceptually separate sections” – true for some designs (e.g., multi-part monographs, tables of authorities), false for many standard “grouped by type/language within one references” layouts where readers still perceive a single bibliography with subheadings.[^9][^12]
- “Users expect ‘2020a’ in each section to be the first item, not a global counter” – this is plausible for independent sections, but the opposite expectation (global stability of labels) is stronger in most general academic contexts.[^1][^2]
- “Legal citation conventions require per-section numbering” – legal authorities often have per-section numbering or grouping requirements, but that cannot be generalized to “year suffix sequence restarts per group” as a universal rule; it is a domain-specific requirement.[^11][^10]


### Implication for the design decision

For a general CSL 2.x / csl26 architecture:

- Treat “year suffix sequence restarts per group” as a **configurable option** (probably off by default for mainstream academic styles).
- Default behavior for broad academic use should keep a single global suffix sequence across all groups within a document, for both citations and references, to match APA/Harvard-type expectations.[^3][^2][^1]
- Provide a way for styles or processors to opt into per-group restarts for cases like legal writing, highly sectionalized documents, or specialized house styles.[^10][^11][^9]

So the problem statement’s *rationale* is valid for specific genres (especially legal and some multi-section works), but as a blanket design decision it overgeneralizes and should be recast as “we support both global and per-group suffix sequencing; legal and some sectional styles will select the latter.”
<span style="display:none">[^13][^14][^15][^16][^17][^18]</span>

<div align="center">⁂</div>

[^1]: https://www.scribbr.com/apa-style/ordering-references/

[^2]: https://libguides.sullivan.edu/apa7/intext

[^3]: https://owl.excelsior.edu/citation-and-documentation/apa-style/apa-in-text-citations/multiple-publications-author-apa-text-citations/

[^4]: https://owl.purdue.edu/owl/research_and_citation/apa_style/apa_formatting_and_style_guide/reference_list_author_authors.html

[^5]: https://onlinelibrary.wiley.com/pb-assets/assets/16000501/APA-Publication-Manual-7th-Edition-by-American-Psychological-Association-1753367708200.pdf

[^6]: https://uclpress.co.uk/author-date-referencing-guidelines/

[^7]: https://yssr.yildiz.edu.tr/instructions-for-authors

[^8]: https://iceb.johogo.com/styles.htm

[^9]: https://www.scribbr.com/citing-sources/citation-styles/

[^10]: https://owl.purdue.edu/owl/research_and_citation/apa_style/apa_formatting_and_style_guide/apa_legal%20references%20.html

[^11]: https://libguides.uakron.edu/c.php?g=627783\&p=5861337

[^12]: https://libguides.reading.ac.uk/citing-references/referencingstyles

[^13]: https://forums.zotero.org/discussion/82401/same-year-same-authors-different-publication-apa-7th-edition

[^14]: https://forums.zotero.org/discussion/99051/open-university-harvard-style-citing-same-author-multiple-times

[^15]: https://gwern.net/style-guide

[^16]: https://forums.zotero.org/discussion/123538/add-year-suffix-by-order-of-citation-occurrence

[^17]: https://hnresearch.lonestar.edu/c.php?g=1004646\&p=10708557

[^18]: https://apastyle.apa.org/style-grammar-guidelines/citations/basic-principles/author-date

