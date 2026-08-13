/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! `check` subcommand: validates style, bibliography, and citation inputs.

use super::CliResult;
use crate::args::{CheckArgs, CheckItem};
use crate::style_resolver::load_any_style;
use citum_io::{load_bibliography, load_citations};

/// Execute the `check` subcommand.
///
/// Attempts to load each provided style, bibliography, and citations file,
/// reporting per-item pass/fail results.  Exits with an error when any check fails.
pub(super) fn run_check(args: CheckArgs) -> CliResult {
    let mut checks = Vec::<CheckItem>::new();

    if let Some(style_input) = args.style {
        let status = check_style_input(&style_input, args.strict);
        checks.push(status);
    }

    for path in args.bibliography {
        let display = path.display().to_string();
        let status = match load_bibliography(&path) {
            Ok(bib) => {
                let mut warnings = Vec::new();
                let class_warnings = citum_engine::api::unknown_reference_class_warnings(&bib);
                warnings.extend(class_warnings.into_iter().map(|w| w.message));

                // Forward-compat: fields captured by the `unknown_fields`
                // catch-all. Hard errors in strict mode, warnings otherwise.
                let field_messages: Vec<String> =
                    citum_engine::api::unknown_reference_field_warnings(&bib)
                        .into_iter()
                        .map(|w| w.message)
                        .collect();

                // For enums, we need a processor context
                let processor = citum_engine::Processor::new(citum_schema::Style::default(), bib);
                let enum_warnings = citum_engine::api::unknown_enum_warnings(&processor);
                warnings.extend(enum_warnings.into_iter().map(|w| w.message));

                let mut ok = true;
                let mut error = None;
                if !field_messages.is_empty() {
                    if args.strict {
                        ok = false;
                        // Each message is a complete sentence; join with a
                        // space rather than "; " to avoid ".;" punctuation.
                        error = Some(format!("strict: {}", field_messages.join(" ")));
                    } else {
                        warnings.extend(field_messages);
                    }
                }

                CheckItem {
                    kind: "bibliography",
                    path: display,
                    ok,
                    schema_version: None,
                    warnings: if warnings.is_empty() {
                        None
                    } else {
                        Some(warnings)
                    },
                    error,
                }
            }
            Err(e) => CheckItem {
                kind: "bibliography",
                path: display,
                ok: false,
                schema_version: None,
                warnings: None,
                error: Some(e.to_string()),
            },
        };
        checks.push(status);
    }

    for path in args.citations {
        let display = path.display().to_string();
        let status = match load_citations(&path) {
            Ok(_) => CheckItem {
                kind: "citations",
                path: display,
                ok: true,
                schema_version: None,
                warnings: None,
                error: None,
            },
            Err(e) => CheckItem {
                kind: "citations",
                path: display,
                ok: false,
                schema_version: None,
                warnings: None,
                error: Some(e.to_string()),
            },
        };
        checks.push(status);
    }

    if args.json {
        println!("{}", serde_json::to_string_pretty(&checks)?);
    } else {
        for check in &checks {
            if check.ok {
                println!("OK   {:<12} {}", check.kind, check.path);
                if let Some(warnings) = &check.warnings {
                    for warn in warnings {
                        println!("  ! {warn}");
                    }
                }
            } else {
                println!("FAIL {:<12} {}", check.kind, check.path);
                if let Some(err) = &check.error {
                    println!("  -> {err}");
                }
            }
        }
    }

    if checks.iter().any(|c| !c.ok) {
        return Err("One or more checks failed.".into());
    }

    Ok(())
}

fn check_style_input(style_input: &str, strict: bool) -> CheckItem {
    let current_version = citum_schema::SchemaVersion::default();
    match load_any_style(style_input, false) {
        Ok(style) => {
            let mut warnings = Vec::new();
            let mut ok = true;
            let mut error = None;

            if style.version.major > current_version.major {
                ok = false;
                error = Some(format!(
                    "Style requires a newer major schema version ({}) than currently supported ({}).",
                    style.version, current_version
                ));
            } else if style.version.major == current_version.major
                && style.version.minor > current_version.minor
            {
                warnings.push(format!(
                    "Style uses a newer minor schema version ({}); some features may not be supported.",
                    style.version
                ));
            }

            // Capture unknown enums and term keys from the style AST
            let processor =
                citum_engine::Processor::new(style.clone(), citum_engine::Bibliography::new());
            let enum_warnings = citum_engine::api::unknown_enum_warnings(&processor);
            warnings.extend(enum_warnings.into_iter().map(|w| w.message));

            let label_warnings =
                citum_engine::api::bibliography_label_missing_separator_warnings(&processor);
            warnings.extend(label_warnings.into_iter().map(|w| w.message));

            // Forward-compat: report captured `unknown_fields` paths. In strict
            // mode every populated path becomes a hard error; otherwise they
            // surface as warnings alongside enum warnings.
            let unknown_paths = citum_engine::api::collect_unknown_field_paths(&style);
            if !unknown_paths.is_empty() {
                let messages: Vec<String> = unknown_paths
                    .into_iter()
                    .map(|p| format!("unknown field(s) at {}: {}", p.path, p.keys.join(", ")))
                    .collect();
                if strict {
                    ok = false;
                    error = Some(format!("strict: {}", messages.join("; ")));
                } else {
                    warnings.extend(messages);
                }
            }

            CheckItem {
                kind: "style",
                path: style_input.to_string(),
                ok,
                schema_version: Some(style.version.to_string()),
                warnings: if warnings.is_empty() {
                    None
                } else {
                    Some(warnings)
                },
                error,
            }
        }
        Err(e) => CheckItem {
            kind: "style",
            path: style_input.to_string(),
            ok: false,
            schema_version: None,
            warnings: None,
            error: Some(e.to_string()),
        },
    }
}
