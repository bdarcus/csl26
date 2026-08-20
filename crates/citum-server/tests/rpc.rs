/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

#![allow(missing_docs, reason = "test")]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::todo,
    clippy::unimplemented,
    clippy::unreachable,
    clippy::get_unwrap,
    reason = "Panicking is acceptable and often desired in test, benchmark, and example code."
)]

//! Integration tests for the JSON-RPC dispatcher.
//!
//! Uses a real Citum style (apa-7th.yaml) and minimal inline reference data
//! to exercise all four methods without touching stdin/stdout.

use citum_server::rpc::{RpcDispatchError, RpcDispatcher, RpcRequest, dispatch};
use serde_json::json;

/// Absolute path to the APA style.
/// `CARGO_MANIFEST_DIR` is the crate root; workspace root is two levels up.
fn apa_style_path() -> String {
    format!(
        "{}/../../styles/embedded/apa-7th.yaml",
        env!("CARGO_MANIFEST_DIR")
    )
}

/// Absolute path to a native Citum YAML bibliography fixture.
fn chicago_bib_path() -> String {
    format!(
        "{}/../../examples/chicago-bib.yaml",
        env!("CARGO_MANIFEST_DIR")
    )
}

/// Minimal bibliography: one book (Hawking 1988) in native Citum schema format.
/// `issued` is a plain EDTF string; `author` is a `ContributorList`.
fn hawking_refs() -> serde_json::Value {
    json!({
        "ITEM-2": {
            "id": "ITEM-2",
            "class": "monograph",
            "type": "book",
            "title": "A Brief History of Time",
            "author": [{"family": "Hawking", "given": "Stephen"}],
            "issued": "1988"
        }
    })
}

fn make_request(id: u32, method: &str, params: serde_json::Value) -> RpcRequest {
    serde_json::from_value(json!({
        "id": id,
        "method": method,
        "params": params
    }))
    .unwrap()
}

// --- validate_style ---

#[test]
fn validate_style_valid() {
    let req = make_request(
        1,
        "validate_style",
        json!({ "style_path": apa_style_path() }),
    );
    let result = dispatch(req).expect("dispatch should succeed");
    assert_eq!(result["id"], 1);
    assert_eq!(result["result"]["valid"], true);
    assert!(result["result"]["warnings"].as_array().unwrap().is_empty());
}

#[test]
fn validate_style_missing_file() {
    let req = make_request(
        2,
        "validate_style",
        json!({ "style_path": "styles/does-not-exist.yaml" }),
    );
    let result = dispatch(req).expect("dispatch should succeed");
    assert_eq!(result["id"], 2);
    assert_eq!(result["result"]["valid"], false);
    assert!(!result["result"]["warnings"].as_array().unwrap().is_empty());
}

// --- render_bibliography ---

#[test]
fn render_bibliography_returns_entries() {
    let req = make_request(
        3,
        "render_bibliography",
        json!({
            "style_path": apa_style_path(),
            "refs": hawking_refs()
        }),
    );
    let result = dispatch(req).expect("dispatch should succeed");
    assert_eq!(result["id"], 3);
    assert_eq!(result["result"]["format"], "plain");
    let entries = result["result"]["entries"]
        .as_array()
        .expect("entries should be array");
    assert!(
        !entries.is_empty(),
        "expected at least one bibliography entry"
    );
    let entry = entries[0].as_str().unwrap();
    assert_eq!(entry, "Hawking, S. (1988). _A Brief History of Time_");
    assert_eq!(
        result["result"]["content"].as_str().unwrap(),
        "Hawking, S. (1988). _A Brief History of Time_"
    );
}

#[test]
fn render_bibliography_html_returns_wrapped_markup() {
    let req = make_request(
        8,
        "render_bibliography",
        json!({
            "style_path": apa_style_path(),
            "refs": hawking_refs(),
            "output_format": "html"
        }),
    );
    let result = dispatch(req).expect("dispatch should succeed");
    assert_eq!(result["id"], 8);
    assert_eq!(result["result"]["format"], "html");
    assert!(result["result"]["entries"].is_null());
    let content = result["result"]["content"]
        .as_str()
        .expect("content should be a string");
    assert!(
        content.contains("citum-bibliography"),
        "html bibliography should include wrapper markup"
    );
}

#[test]
fn render_bibliography_html_injects_template_indices_when_requested() {
    let req = make_request(
        13,
        "render_bibliography",
        json!({
            "style_path": apa_style_path(),
            "refs": hawking_refs(),
            "output_format": "html",
            "inject_ast_indices": true
        }),
    );
    let result = dispatch(req).expect("dispatch should succeed");
    let content = result["result"]["content"]
        .as_str()
        .expect("content should be a string");
    assert!(
        content.contains(r#"data-index="0""#),
        "html bibliography should include template indices when requested: {content}"
    );
}

// --- render_citation ---

#[test]
fn render_citation_returns_string() {
    let req = make_request(
        4,
        "render_citation",
        json!({
            "style_path": apa_style_path(),
            "refs": hawking_refs(),
            "citation": {
                "id": "cite-1",
                "items": [{"id": "ITEM-2"}]
            }
        }),
    );
    let result = dispatch(req).expect("dispatch should succeed");
    assert_eq!(result["id"], 4);
    let citation = result["result"].as_str().expect("result should be string");
    assert!(
        citation.contains("Hawking") || citation.contains("1988"),
        "citation should reference the work: {citation}"
    );
}

#[test]
fn render_citation_html_returns_markup() {
    let req = make_request(
        9,
        "render_citation",
        json!({
            "style_path": apa_style_path(),
            "refs": hawking_refs(),
            "output_format": "html",
            "citation": {
                "id": "cite-1",
                "items": [{"id": "ITEM-2"}]
            }
        }),
    );
    let result = dispatch(req).expect("dispatch should succeed");
    assert_eq!(result["id"], 9);
    let citation = result["result"].as_str().expect("result should be string");
    assert!(
        citation.contains("citum-citation"),
        "html citation should contain citation wrapper: {citation}"
    );
}

#[test]
fn render_citation_html_injects_template_indices_when_requested() {
    let req = make_request(
        14,
        "render_citation",
        json!({
            "style_path": apa_style_path(),
            "refs": hawking_refs(),
            "output_format": "html",
            "inject_ast_indices": true,
            "citation": {
                "id": "cite-1",
                "items": [{"id": "ITEM-2"}]
            }
        }),
    );
    let result = dispatch(req).expect("dispatch should succeed");
    let citation = result["result"].as_str().expect("result should be string");
    assert!(
        citation.contains(r#"class="citum-issued" data-index="0""#),
        "html citation should annotate the rendered citation component when requested: {citation}"
    );
}

#[test]
fn render_citation_typst_returns_internal_link_markup() {
    let req = make_request(
        11,
        "render_citation",
        json!({
            "style_path": apa_style_path(),
            "refs": hawking_refs(),
            "output_format": "typst",
            "citation": {
                "id": "cite-1",
                "items": [{"id": "ITEM-2"}]
            }
        }),
    );
    let result = dispatch(req).expect("dispatch should succeed");
    assert_eq!(result["id"], 11);
    let citation = result["result"].as_str().expect("result should be string");
    assert!(
        citation.contains("#link(<ref-ITEM-2>)"),
        "typst citation should contain an internal link: {citation}"
    );
}

// --- format_document ---

#[test]
fn format_document_returns_citations_bibliography_and_warnings() {
    let req = make_request(
        15,
        "format_document",
        json!({
            "style": {
                "kind": "path",
                "value": apa_style_path()
            },
            "output_format": "html",
            "refs": hawking_refs(),
            "citations": [{
                "id": "cite-1",
                "items": [{"id": "ITEM-2"}]
            }],
            "document_options": {
                "show_semantics": true
            }
        }),
    );

    let result = dispatch(req).expect("dispatch should succeed");
    assert_eq!(result["id"], 15);

    let payload = &result["result"];
    assert!(
        payload["formatted_citations"].is_array(),
        "document result should include formatted_citations: {payload}"
    );
    assert!(
        payload["bibliography"].is_object(),
        "document result should include bibliography: {payload}"
    );
    assert!(
        payload["warnings"].is_array(),
        "document result should include warnings: {payload}"
    );

    let formatted_citations = payload["formatted_citations"]
        .as_array()
        .expect("formatted_citations should be an array");
    assert_eq!(formatted_citations.len(), 1);
    assert_eq!(formatted_citations[0]["id"], "cite-1");

    let bibliography = &payload["bibliography"];
    assert_eq!(bibliography["format"], "html");
    let content = bibliography["content"]
        .as_str()
        .expect("bibliography.content should be a string");
    // APA 7th declares `hanging-indent: true`
    // (crates/citum-schema-style/embedded/styles/apa-7th.yaml), so the
    // container carries the hanging-indent modifier class — see
    // docs/specs/SECOND_FIELD_ALIGN.md.
    assert!(
        content.contains(r#"<div class="citum-bibliography citum-bibliography--hanging-indent">"#),
        "bibliography.content should contain rendered bibliography markup: {content}"
    );

    let entries = bibliography["entries"]
        .as_array()
        .expect("bibliography.entries should be an array");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0]["id"], "ITEM-2");
}

#[test]
fn format_document_accepts_refs_path_native_bibliography() {
    let req = make_request(
        16,
        "format_document",
        json!({
            "style": {
                "kind": "path",
                "value": apa_style_path()
            },
            "output_format": "plain",
            "refs": {
                "kind": "path",
                "value": chicago_bib_path()
            },
            "citations": [{
                "id": "cite-1",
                "items": [{"id": "biss"}]
            }]
        }),
    );

    let result = dispatch(req).expect("dispatch should succeed");
    assert_eq!(result["id"], 16);

    let payload = &result["result"];
    let formatted_citations = payload["formatted_citations"]
        .as_array()
        .expect("formatted_citations should be an array");
    assert_eq!(formatted_citations.len(), 1);
    assert_eq!(formatted_citations[0]["id"], "cite-1");

    let entries = payload["bibliography"]["entries"]
        .as_array()
        .expect("bibliography.entries should be an array");
    assert!(
        entries.iter().any(|entry| entry["id"] == "biss"),
        "bibliography should include the path-loaded reference: {payload}"
    );
}

// --- sessions ---

#[test]
fn stdio_dispatcher_keeps_default_session_across_requests() {
    let mut dispatcher = RpcDispatcher::new_stdio();
    let opened = dispatcher
        .dispatch(make_request(
            17,
            "open_session",
            json!({
                "style": {
                    "kind": "path",
                    "value": apa_style_path()
                },
                "output_format": "html"
            }),
        ))
        .expect("open_session should succeed");
    assert_eq!(opened["result"]["session_id"], "default");

    dispatcher
        .dispatch(make_request(
            18,
            "put_references",
            json!({
                "session_id": "default",
                "refs": hawking_refs()
            }),
        ))
        .expect("put_references should succeed");

    let inserted = dispatcher
        .dispatch(make_request(
            19,
            "insert_citation",
            json!({
                "session_id": "default",
                "citation": {
                    "id": "cite-1",
                    "items": [{"id": "ITEM-2"}]
                }
            }),
        ))
        .expect("insert_citation should succeed");
    assert_eq!(inserted["result"]["version"], 1);
    assert_eq!(inserted["result"]["affected_citations"][0]["id"], "cite-1");
    assert_eq!(inserted["result"]["renumbering_occurred"], false);

    let citations = dispatcher
        .dispatch(make_request(
            20,
            "get_citations",
            json!({ "session_id": "default" }),
        ))
        .expect("get_citations should succeed");
    assert_eq!(
        citations["result"]["formatted_citations"][0]["id"],
        "cite-1"
    );
}

#[test]
fn put_references_with_malformed_refs_errors_at_put_time() {
    let mut dispatcher = RpcDispatcher::new_stdio();
    dispatcher
        .dispatch(make_request(
            30,
            "open_session",
            json!({
                "style": {
                    "kind": "path",
                    "value": apa_style_path()
                }
            }),
        ))
        .expect("open_session should succeed");

    let err = dispatcher
        .dispatch(make_request(
            31,
            "put_references",
            json!({
                "session_id": "default",
                "refs": { "kind": "yaml", "value": "not: [valid" }
            }),
        ))
        .expect_err("malformed refs should error when they are put, not on the next mutation");
    assert_eq!(err.0, Some(json!(31)));
}

#[test]
fn http_dispatcher_generates_independent_session_ids() {
    let mut dispatcher = RpcDispatcher::new_http();
    let first = dispatcher
        .dispatch(make_request(
            21,
            "open_session",
            json!({
                "style": {
                    "kind": "path",
                    "value": apa_style_path()
                }
            }),
        ))
        .expect("first open_session should succeed");
    let second = dispatcher
        .dispatch(make_request(
            22,
            "open_session",
            json!({
                "style": {
                    "kind": "path",
                    "value": apa_style_path()
                }
            }),
        ))
        .expect("second open_session should succeed");

    assert_ne!(
        first["result"]["session_id"],
        second["result"]["session_id"]
    );
}

#[test]
fn http_dispatcher_requires_session_id_for_session_lookup() {
    let mut dispatcher = RpcDispatcher::new_http();
    let err = dispatcher
        .dispatch(make_request(23, "get_citations", json!({})))
        .expect_err("missing HTTP session_id should error");
    let message = match err.1 {
        RpcDispatchError::Message(message) => message,
        RpcDispatchError::Response(response) => response.to_string(),
    };

    assert_eq!(err.0, Some(json!(23)));
    assert_eq!(message, "missing required field: session_id");
}

#[test]
fn http_dispatcher_requires_session_id_for_close_session() {
    let mut dispatcher = RpcDispatcher::new_http();
    let err = dispatcher
        .dispatch(make_request(24, "close_session", json!({})))
        .expect_err("missing HTTP session_id should error");
    let message = match err.1 {
        RpcDispatchError::Message(message) => message,
        RpcDispatchError::Response(response) => response.to_string(),
    };

    assert_eq!(err.0, Some(json!(24)));
    assert_eq!(message, "missing required field: session_id");
}

// --- error handling ---

#[test]
fn unknown_method_returns_error() {
    let req = make_request(5, "frobnicate", json!({}));
    let err = dispatch(req).expect_err("should error");
    assert_eq!(err.1, "unknown method: frobnicate");
}

#[test]
fn missing_style_path_returns_error() {
    let req = make_request(6, "render_bibliography", json!({ "refs": hawking_refs() }));
    let err = dispatch(req).expect_err("should error");
    assert_eq!(err.1, "missing required field: style_path");
}

#[test]
fn missing_refs_returns_error() {
    let req = make_request(
        7,
        "render_bibliography",
        json!({ "style_path": apa_style_path() }),
    );
    let err = dispatch(req).expect_err("should error");
    assert_eq!(err.1, "missing required field: refs");
}

#[test]
fn invalid_output_format_returns_error() {
    let req = make_request(
        12,
        "render_bibliography",
        json!({
            "style_path": apa_style_path(),
            "refs": hawking_refs(),
            "output_format": "pdf"
        }),
    );
    let err = dispatch(req).expect_err("invalid output format should error");
    assert_eq!(err.0, Some(json!(12)));
    assert!(err.1.contains("unsupported output format: pdf"));
}

#[test]
fn render_bibliography_typst_returns_labeled_markup() {
    let req = make_request(
        10,
        "render_bibliography",
        json!({
            "style_path": apa_style_path(),
            "refs": hawking_refs(),
            "output_format": "typst"
        }),
    );
    let result = dispatch(req).expect("dispatch should succeed");
    assert_eq!(result["id"], 10);
    assert_eq!(result["result"]["format"], "typst");
    assert!(result["result"]["entries"].is_null());
    let content = result["result"]["content"]
        .as_str()
        .expect("content should be a string");
    assert_eq!(
        content,
        "Hawking, S. (1988). #emph[A Brief History of Time] <ref-ITEM-2>"
    );
}

/// Two-author bibliography for testing the `and` connector override.
fn duo_refs() -> serde_json::Value {
    json!({
        "DUO-1": {
            "id": "DUO-1",
            "class": "monograph",
            "type": "book",
            "title": "Collaborative Work",
            "author": [
                {"family": "Smith", "given": "Alice"},
                {"family": "Jones", "given": "Bob"}
            ],
            "issued": "2024"
        }
    })
}

// --- style_overrides ---

#[test]
fn format_document_style_overrides_changes_and_connector() {
    // APA uses `&` by default; override to `text` should produce "and".
    let req_base = make_request(
        60,
        "format_document",
        json!({
            "style": {"kind": "path", "value": apa_style_path()},
            "refs": duo_refs(),
            "citations": [{"id": "cite-1", "items": [{"id": "DUO-1"}]}]
        }),
    );
    let result_base = dispatch(req_base).expect("base format_document should succeed");
    let text_base = result_base["result"]["formatted_citations"][0]["text"]
        .as_str()
        .expect("citation text should be a string");
    assert!(
        text_base.contains('&'),
        "APA base style should use '&' connector, got: {text_base:?}"
    );

    // With style_overrides switching to text "and"
    let req_override = make_request(
        61,
        "format_document",
        json!({
            "style": {"kind": "path", "value": apa_style_path()},
            "style_overrides": "options:\n  contributors:\n    and: text\n",
            "refs": duo_refs(),
            "citations": [{"id": "cite-1", "items": [{"id": "DUO-1"}]}]
        }),
    );
    let result_override = dispatch(req_override).expect("override format_document should succeed");
    let text_override = result_override["result"]["formatted_citations"][0]["text"]
        .as_str()
        .expect("citation text should be a string");
    assert!(
        !text_override.contains('&'),
        "overridden style should not use '&' connector, got: {text_override:?}"
    );
    assert_ne!(
        text_base, text_override,
        "overridden and base outputs should differ"
    );
}

#[test]
fn open_session_style_overrides_changes_and_connector() {
    let mut dispatcher = RpcDispatcher::new_stdio();

    // Session opened with `and: text` override
    dispatcher
        .dispatch(make_request(
            62,
            "open_session",
            json!({
                "style": {"kind": "path", "value": apa_style_path()},
                "style_overrides": "options:\n  contributors:\n    and: text\n"
            }),
        ))
        .expect("open_session with style_overrides should succeed");

    dispatcher
        .dispatch(make_request(
            63,
            "put_references",
            json!({"session_id": "default", "refs": duo_refs()}),
        ))
        .expect("put_references should succeed");

    let inserted = dispatcher
        .dispatch(make_request(
            64,
            "insert_citation",
            json!({
                "session_id": "default",
                "citation": {"id": "cite-1", "items": [{"id": "DUO-1"}]}
            }),
        ))
        .expect("insert_citation should succeed");

    let text = inserted["result"]["affected_citations"][0]["text"]
        .as_str()
        .expect("citation text should be a string");
    assert!(
        !text.contains('&'),
        "session with and:text override should not use '&', got: {text:?}"
    );
}

// --- set_nocite ---

fn two_refs() -> serde_json::Value {
    json!({
        "ITEM-2": {
            "id": "ITEM-2",
            "class": "monograph",
            "type": "book",
            "title": "A Brief History of Time",
            "author": [{"family": "Hawking", "given": "Stephen"}],
            "issued": "1988"
        },
        "ITEM-3": {
            "id": "ITEM-3",
            "class": "monograph",
            "type": "book",
            "title": "Cosmos",
            "author": [{"family": "Sagan", "given": "Carl"}],
            "issued": "1980"
        }
    })
}

#[test]
fn set_nocite_puts_ref_in_bibliography_not_in_formatted_citations() {
    // given: a session with ITEM-2 cited in-text and ITEM-3 registered as nocite
    let mut dispatcher = RpcDispatcher::new_stdio();
    dispatcher
        .dispatch(make_request(
            70,
            "open_session",
            json!({
                "style": {"kind": "path", "value": apa_style_path()}
            }),
        ))
        .expect("open_session should succeed");

    dispatcher
        .dispatch(make_request(
            71,
            "put_references",
            json!({"session_id": "default", "refs": two_refs()}),
        ))
        .expect("put_references should succeed");

    dispatcher
        .dispatch(make_request(
            72,
            "insert_citation",
            json!({
                "session_id": "default",
                "citation": {"id": "cite-1", "items": [{"id": "ITEM-2"}]}
            }),
        ))
        .expect("insert_citation should succeed");

    // when: ITEM-3 is registered as nocite
    let result = dispatcher
        .dispatch(make_request(
            73,
            "set_nocite",
            json!({"session_id": "default", "nocite": ["ITEM-3"]}),
        ))
        .expect("set_nocite should succeed");

    // then: ITEM-3 appears in bibliography entries but not in formatted citations
    let entries = &result["result"]["bibliography"]["entries"];
    let ids: Vec<&str> = entries
        .as_array()
        .expect("entries should be an array")
        .iter()
        .map(|e| e["id"].as_str().expect("entry id should be a string"))
        .collect();

    assert!(
        ids.contains(&"ITEM-3"),
        "nocite ref ITEM-3 should appear in bibliography entries, got: {ids:?}"
    );
    let formatted = &result["result"]["affected_citations"];
    let any_item3 = formatted
        .as_array()
        .map(|cites| {
            cites.iter().any(|c| {
                c["ref_ids"]
                    .as_array()
                    .map(|ids| ids.iter().any(|id| id == "ITEM-3"))
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false);
    assert!(
        !any_item3,
        "nocite ref ITEM-3 should not appear in any formatted citation"
    );
}
