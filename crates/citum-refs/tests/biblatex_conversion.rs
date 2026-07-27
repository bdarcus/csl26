/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

#![allow(missing_docs, reason = "test")]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "Panicking is acceptable and desired in test code."
)]

//! Contract tests for the BibLaTeX conversion path (bean csl26-11h2), driven
//! by real, multi-field `.bib` fixtures rather than the single-feature
//! entries `citum-refs/src/formats/biblatex/mapping.rs`'s inline unit tests
//! use. Each fixture goes through `load_input_refs` -- the same entry point
//! `citum convert refs` uses -- so this exercises the whole path, not just
//! `input_reference_from_biblatex` in isolation. Gives the
//! exact-match-vs-Zotero gap (previously tracked only by the external,
//! unpersisted gb7714-bench CI artifact) something local to regress against.

use std::path::PathBuf;

use citum_refs::{RefsFormat, load_input_refs};
use citum_schema::InputBibliography;
use citum_schema::reference::{InputReference, Title};
use rstest::rstest;

fn fixture_path(name: &str) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests/fixtures/biblatex");
    path.push(name);
    path
}

const BOOKS_YAML: &str = "references:\n\
- class: monograph\n  \
  id: turing1936\n  \
  type: book\n  \
  title: On Computable Numbers, with an Application to the Entscheidungsproblem\n  \
  container:\n    \
    class: monograph\n    \
    type: book\n    \
    container:\n      \
      class: collection\n      \
      type: anthology\n      \
      title: Proceedings of the London Mathematical Society\n  \
  contributors:\n  \
  - roles:\n    \
    - author\n    \
    contributor:\n      \
      given: Alan M.\n      \
      family: Turing\n  \
  issued: '1936'\n  \
  publisher:\n    \
    name: London Mathematical Society\n    \
    place: London\n  \
  numbering:\n  \
  - type: volume\n    \
    value: '42'\n\
- class: collection\n  \
  id: shannon1993\n  \
  type: edited-book\n  \
  title: 'Claude Elwood Shannon: Collected Papers'\n  \
  container:\n    \
    class: collection\n    \
    type: anthology\n    \
    title: IEEE Information Theory Series\n  \
  contributors:\n  \
  - roles:\n    \
    - editor\n    \
    contributor:\n    \
    - given: N. J. A.\n      \
      family: Sloane\n    \
    - given: Aaron D.\n      \
      family: Wyner\n  \
  issued: '1993'\n  \
  publisher:\n    \
    name: IEEE Press\n    \
    place: New York\n  \
  numbering:\n  \
  - type: volume\n    \
    value: '7'\n  \
  isbn: 978-0-7803-0434-5\n\
- class: collection\n  \
  id: icml2024\n  \
  type: proceedings\n  \
  title: Proceedings of the 41st International Conference on Machine Learning\n  \
  contributors:\n  \
  - roles:\n    \
    - editor\n    \
    contributor:\n      \
      given: Jane\n      \
      family: Doe\n  \
  issued: '2024'\n  \
  publisher:\n    \
    name: PMLR\n    \
    place: Vienna\n  \
  event:\n    \
    class: event\n    \
    title: International Conference on Machine Learning\n    \
    location: Vienna, Austria\n    \
    date: 2024-07-21\n";

const PARTS_YAML: &str = "references:\n\
- class: collection-component\n  \
  id: turing1950\n  \
  type: chapter\n  \
  title: Computing Machinery and Intelligence\n  \
  contributors:\n  \
  - roles:\n    \
    - author\n    \
    contributor:\n      \
      given: Alan M.\n      \
      family: Turing\n  \
  issued: '1950'\n  \
  container:\n    \
    class: collection\n    \
    type: edited-book\n    \
    title: 'The Mind''s I: Fantasies and Reflections on Self and Soul'\n    \
    container:\n      \
      class: collection\n      \
      type: anthology\n      \
      title: Bantam New Age Books\n    \
    contributors:\n    \
    - roles:\n      \
      - editor\n      \
      contributor:\n      \
      - given: Douglas R.\n        \
        family: Hofstadter\n      \
      - given: Daniel C.\n        \
        family: Dennett\n    \
    publisher:\n      \
      name: Basic Books\n  \
  pages: 53-67\n\
- class: collection-component\n  \
  id: lecun1998\n  \
  type: document\n  \
  title: Gradient-Based Learning Applied to Document Recognition\n  \
  contributors:\n  \
  - roles:\n    \
    - author\n    \
    contributor:\n    \
    - given: Yann\n      \
      family: LeCun\n    \
    - given: Léon\n      \
      family: Bottou\n  \
  issued: '1998'\n  \
  container:\n    \
    class: collection\n    \
    type: proceedings\n    \
    title: Proceedings of the IEEE\n    \
    event:\n      \
      class: event\n      \
      title: IEEE Conference on Neural Networks\n      \
      location: Anchorage, AK\n      \
      date: 1998-05-04\n  \
  numbering:\n  \
  - type: chapter\n    \
    value: '11'\n  \
  pages: 2278-2324\n";

const SERIALS_YAML: &str = "references:\n\
- class: serial-component\n  \
  id: shannon1948\n  \
  type: article\n  \
  title: A Mathematical Theory of Communication\n  \
  contributors:\n  \
  - roles:\n    \
    - author\n    \
    contributor:\n      \
      given: Claude E.\n      \
      family: Shannon\n  \
  issued: '1948'\n  \
  container:\n    \
    class: serial\n    \
    type: academic-journal\n    \
    title: The Bell System Technical Journal\n  \
  numbering:\n  \
  - type: volume\n    \
    value: '27'\n  \
  - type: issue\n    \
    value: '3'\n  \
  pages: 379-423\n\
- class: monograph\n  \
  id: arxiv2301\n  \
  type: preprint\n  \
  title: A Preprint Without a Journal\n  \
  contributors:\n  \
  - roles:\n    \
    - author\n    \
    contributor:\n      \
      given: Jane\n      \
      family: Doe\n  \
  issued: '2023'\n  \
  eprint:\n    \
    id: '2301.00001'\n    \
    server: arxiv\n    \
    class: cs.CL\n\
- class: monograph\n  \
  id: plos2024\n  \
  type: document\n  \
  title: PLOS Computational Biology\n  \
  contributors:\n  \
  - roles:\n    \
    - editor\n    \
    contributor:\n      \
      given: John\n      \
      family: Smith\n  \
  issued: '2024'\n  \
  genre: periodical\n";

const REPORTS_THESES_YAML: &str = "references:\n\
- class: monograph\n  \
  id: rfc791\n  \
  type: report\n  \
  title: Internet Protocol\n  \
  contributors:\n  \
  - roles:\n    \
    - author\n    \
    contributor:\n      \
      given: Jon\n      \
      family: Postel\n  \
  issued: '1981'\n  \
  publisher:\n    \
    name: DARPA\n  \
  numbering:\n  \
  - type: report\n    \
    value: RFC 791\n\
- class: monograph\n  \
  id: hinton1978\n  \
  type: thesis\n  \
  title: Relaxation and Its Role in Vision\n  \
  contributors:\n  \
  - roles:\n    \
    - author\n    \
    contributor:\n      \
      given: Geoffrey E.\n      \
      family: Hinton\n  \
  issued: '1978'\n  \
  publisher:\n    \
    name: University of Edinburgh\n    \
    place: Edinburgh\n\
- class: monograph\n  \
  id: w3c-html5\n  \
  type: webpage\n  \
  title: 'HTML: Living Standard'\n  \
  contributors:\n  \
  - roles:\n    \
    - author\n    \
    contributor:\n      \
      given: World Wide Web\n      \
      family: Consortium\n  \
  issued: '2024'\n  \
  url: https://html.spec.whatwg.org/\n  \
  accessed: 2024-01-15\n\
- class: monograph\n  \
  id: draft2024\n  \
  type: manuscript\n  \
  title: A Working Paper on Citation Graphs\n  \
  contributors:\n  \
  - roles:\n    \
    - author\n    \
    contributor:\n      \
      given: Sam\n      \
      family: Roe\n  \
  issued: '2024'\n  \
  note: Manuscript in preparation\n";

const SPECIALIZED_YAML: &str = "references:\n\
- class: patent\n  \
  id: us7347809\n  \
  title: Method and System for Searching a Distributed Database\n  \
  author:\n    \
    given: Lawrence\n    \
    family: Page\n  \
  assignee:\n    \
    name: Google LLC\n  \
  patent-number: US 7,347,809 B2\n  \
  issued: '2008'\n  \
  jurisdiction: US\n\
- class: dataset\n  \
  id: gutenberg2024\n  \
  title: Project Gutenberg Text Corpus\n  \
  author:\n    \
    name: Project Gutenberg\n  \
  issued: '2024'\n  \
  publisher:\n    \
    name: Zenodo\n  \
  version: '2024.1'\n\
- class: software\n  \
  id: numpy2020\n  \
  title: NumPy\n  \
  author:\n    \
    given: Charles R.\n    \
    family: Harris\n  \
  issued: '2020'\n  \
  version: 1.19.0\n  \
  url: https://numpy.org/\n\
- class: standard\n  \
  id: iso8601\n  \
  title: Date and Time – Representations for Information Interchange\n  \
  authority: ISO\n  \
  standard-number: ISO 8601-1:2019\n  \
  issued: '2019'\n  \
  publisher:\n    \
    name: ISO\n";

const CONTRIBUTORS_YAML: &str = "references:\n\
- class: monograph\n  \
  id: plato-republic\n  \
  type: book\n  \
  title: The Republic\n  \
  contributors:\n  \
  - roles:\n    \
    - introduction-author\n    \
    contributor:\n      \
      given: Francis M.\n      \
      family: Cornford\n  \
  - roles:\n    \
    - author\n    \
    contributor:\n      \
      name: Plato\n  \
  - roles:\n    \
    - translator\n    \
    contributor:\n      \
      given: Benjamin\n      \
      family: Jowett\n  \
  issued: '1941'\n  \
  publisher:\n    \
    name: Oxford University Press\n\
- class: collection-component\n  \
  id: critical-edition\n  \
  type: chapter\n  \
  title: Moby-Dick\n  \
  contributors:\n  \
  - roles:\n    \
    - compiler\n    \
    contributor:\n      \
      given: Hershel\n      \
      family: Parker\n  \
  - roles:\n    \
    - annotator\n    \
    contributor:\n      \
      given: John\n      \
      family: Bryant\n  \
  - roles:\n    \
    - author\n    \
    contributor:\n      \
      given: Herman\n      \
      family: Melville\n  \
  issued: '1988'\n  \
  container:\n    \
    class: collection\n    \
    type: edited-book\n    \
    title: The Northwestern-Newberry Edition\n    \
    contributors:\n    \
    - roles:\n      \
      - editor\n      \
      contributor:\n        \
        given: Harrison\n        \
        family: Hayford\n";

const ZOTERO_SHAPES_YAML: &str = "references:\n\
- class: monograph\n  \
  id: eucalyptus-genome\n  \
  type: book\n  \
  title: The genome of [Eucalyptus grandis]{.nocase}\n  \
  contributors:\n  \
  - roles:\n    \
    - author\n    \
    contributor:\n      \
      given: Alexander A.\n      \
      family: Myburg\n  \
  issued: '2014'\n\
- class: collection-component\n  \
  id: holography-symposium\n  \
  type: document\n  \
  title: Advances in holographic photoelasticity\n  \
  contributors:\n  \
  - roles:\n    \
    - author\n    \
    contributor:\n      \
      given: Charles M.\n      \
      family: Vest\n  \
  issued: '1971'\n  \
  container:\n    \
    class: collection\n    \
    type: proceedings\n    \
    title: '[Symposium on Applications of Holography in Mechanics]{.nocase}'\n";

#[rstest]
#[case::books("books.bib", BOOKS_YAML)]
#[case::parts("parts.bib", PARTS_YAML)]
#[case::serials("serials.bib", SERIALS_YAML)]
#[case::reports_theses("reports-theses.bib", REPORTS_THESES_YAML)]
#[case::specialized("specialized.bib", SPECIALIZED_YAML)]
#[case::contributors("contributors.bib", CONTRIBUTORS_YAML)]
#[case::zotero_shapes("zotero-shapes.bib", ZOTERO_SHAPES_YAML)]
fn given_a_biblatex_fixture_when_loaded_then_matches_the_golden_yaml(
    #[case] fixture: &str,
    #[case] golden: &str,
) {
    let bibliography = load_input_refs(&fixture_path(fixture), RefsFormat::Biblatex)
        .expect("fixture should convert");
    let yaml = serde_yaml::to_string(&bibliography).expect("bibliography should serialize");

    assert_eq!(yaml, golden);
}

fn reference_by_id<'a>(bibliography: &'a InputBibliography, id: &str) -> &'a InputReference {
    bibliography
        .references
        .iter()
        .find(|reference| {
            reference
                .id()
                .is_some_and(|candidate| candidate.as_str() == id)
        })
        .unwrap_or_else(|| panic!("fixture should contain reference {id}"))
}

#[test]
fn fixture_corpus_preserves_reference_semantics() {
    let parts = load_input_refs(&fixture_path("parts.bib"), RefsFormat::Biblatex)
        .expect("parts fixture should convert");
    assert_eq!(
        reference_by_id(&parts, "lecun1998").ref_type(),
        "paper-conference"
    );

    let serials = load_input_refs(&fixture_path("serials.bib"), RefsFormat::Biblatex)
        .expect("serials fixture should convert");
    let periodical = reference_by_id(&serials, "plos2024");
    assert_eq!(periodical.ref_type(), "periodical");
    assert!(
        periodical.issued().is_some(),
        "periodical date should survive"
    );

    let books = load_input_refs(&fixture_path("books.bib"), RefsFormat::Biblatex)
        .expect("books fixture should convert");
    let collection = reference_by_id(&books, "shannon1993");
    assert_eq!(
        collection.collection_title(),
        Some(Title::Single("IEEE Information Theory Series".to_string()))
    );
    assert_eq!(collection.collection_number().as_deref(), Some("7"));
}
