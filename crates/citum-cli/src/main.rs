/*
SPDX-License-Identifier: MIT OR Apache-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus and Citum contributors
*/

//! Entry point for the `citum` command-line interface.
//!
//! This binary wires the top-level CLI commands and delegates their work to
//! the library crates.

#![allow(missing_docs, reason = "bin")]

mod args;
mod commands;
mod output;
mod style_browser;
mod style_catalog;
mod style_resolver;
mod table;
mod typst_pdf;

#[cfg(feature = "dhat-heap")]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

#[cfg(feature = "dhat-heap")]
fn heap_profiling_requested(value: Option<&std::ffi::OsStr>) -> bool {
    value.is_some()
}

fn main() {
    #[cfg(feature = "dhat-heap")]
    let _profiler = heap_profiling_requested(std::env::var_os("CITUM_DHAT_HEAP").as_deref())
        .then(dhat::Profiler::new_heap);

    if let Err(e) = commands::run() {
        eprintln!("\nError: {e}");
        std::process::exit(1);
    }
}

#[cfg(all(test, feature = "dhat-heap"))]
mod tests {
    use super::heap_profiling_requested;
    use std::ffi::OsStr;

    #[test]
    fn heap_profiling_requires_explicit_environment_opt_in() {
        assert!(!heap_profiling_requested(None));
        assert!(heap_profiling_requested(Some(OsStr::new("1"))));
    }
}
