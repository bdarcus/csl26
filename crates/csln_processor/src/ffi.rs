/*
SPDX-License-Identifier: MPL-2.0
SPDX-FileCopyrightText: © 2023-2026 Bruce D'Arcus
*/

#![allow(unsafe_code)]

//! C-FFI for the CSLN processor.
//!
//! This module provides a C-compatible interface for other languages
//! (like Lua, Python, or JavaScript) to use the processor.

use crate::processor::Processor;
use crate::reference::{Bibliography, Citation, Reference};
use crate::render::html::Html;
use crate::render::latex::Latex;
use crate::render::plain::PlainText;
use csln_core::Style;
use csln_core::locale::Locale;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;

/// Helper to safely create a C string from a Rust string, returning null if it contains null bytes.
fn safe_c_string(s: String) -> *mut c_char {
    match CString::new(s) {
        Ok(c) => c.into_raw(),
        Err(_) => ptr::null_mut(),
    }
}

/// Create a new processor instance from JSON strings with default English locale.
///
/// # Safety
/// The caller must ensure that `style_json` and `bib_json` are valid
/// null-terminated C strings. The returned pointer must be freed
/// with `csln_processor_free`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn csln_processor_new(
    style_json: *const c_char,
    bib_json: *const c_char,
) -> *mut Processor {
    if style_json.is_null() || bib_json.is_null() {
        return ptr::null_mut();
    }

    let style_str = match unsafe { CStr::from_ptr(style_json) }.to_str() {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };

    let bib_str = match unsafe { CStr::from_ptr(bib_json) }.to_str() {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };

    let style: Style = match serde_json::from_str(style_str) {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };

    // Try parsing as CSL-JSON bibliography first
    let bib: Bibliography =
        match serde_json::from_str::<Vec<csl_legacy::csl_json::Reference>>(bib_str) {
            Ok(legacy_refs) => legacy_refs
                .into_iter()
                .map(|r| (r.id.clone(), Reference::from(r)))
                .collect(),
            Err(_) => match serde_json::from_str(bib_str) {
                Ok(b) => b,
                Err(_) => return ptr::null_mut(),
            },
        };

    let processor = Box::new(Processor::new(style, bib));
    Box::into_raw(processor)
}

/// Create a new processor instance with a specific locale.
///
/// # Safety
/// The caller must ensure all string pointers are valid null-terminated C strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn csln_processor_new_with_locale(
    style_json: *const c_char,
    bib_json: *const c_char,
    locale_json: *const c_char,
) -> *mut Processor {
    if style_json.is_null() || bib_json.is_null() || locale_json.is_null() {
        return ptr::null_mut();
    }

    let style_str = match unsafe { CStr::from_ptr(style_json) }.to_str() {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };

    let bib_str = match unsafe { CStr::from_ptr(bib_json) }.to_str() {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };

    let locale_str = match unsafe { CStr::from_ptr(locale_json) }.to_str() {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };

    let style: Style = match serde_json::from_str(style_str) {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };

    let bib: Bibliography =
        match serde_json::from_str::<Vec<csl_legacy::csl_json::Reference>>(bib_str) {
            Ok(legacy_refs) => legacy_refs
                .into_iter()
                .map(|r| (r.id.clone(), Reference::from(r)))
                .collect(),
            Err(_) => match serde_json::from_str(bib_str) {
                Ok(b) => b,
                Err(_) => return ptr::null_mut(),
            },
        };

    let locale: Locale = match serde_json::from_str(locale_str) {
        Ok(l) => l,
        Err(_) => return ptr::null_mut(),
    };

    let processor = Box::new(Processor::with_locale(style, bib, locale));
    Box::into_raw(processor)
}

/// Free a processor instance.
///
/// # Safety
/// The pointer must have been created by a `csln_processor_new` function.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn csln_processor_free(processor: *mut Processor) {
    if !processor.is_null() {
        let _ = unsafe { Box::from_raw(processor) };
    }
}

/// Render a citation to a LaTeX string.
///
/// # Safety
/// The caller must ensure that `processor` is a valid pointer and
/// `cite_json` is a valid null-terminated C string. The returned
/// string must be freed with `csln_string_free`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn csln_render_citation_latex(
    processor: *mut Processor,
    cite_json: *const c_char,
) -> *mut c_char {
    if processor.is_null() || cite_json.is_null() {
        return ptr::null_mut();
    }

    let processor = unsafe { &*processor };
    let cite_str = match unsafe { CStr::from_ptr(cite_json) }.to_str() {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };

    let citation: Citation = match serde_json::from_str(cite_str) {
        Ok(c) => c,
        Err(_) => return ptr::null_mut(),
    };

    match processor.process_citation_with_format::<Latex>(&citation) {
        Ok(rendered) => safe_c_string(rendered),
        Err(_) => ptr::null_mut(),
    }
}

/// Render a citation to an HTML string.
///
/// # Safety
/// The caller must ensure that `processor` is a valid pointer and
/// `cite_json` is a valid null-terminated C string. The returned
/// string must be freed with `csln_string_free`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn csln_render_citation_html(
    processor: *mut Processor,
    cite_json: *const c_char,
) -> *mut c_char {
    if processor.is_null() || cite_json.is_null() {
        return ptr::null_mut();
    }

    let processor = unsafe { &*processor };
    let cite_str = match unsafe { CStr::from_ptr(cite_json) }.to_str() {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };

    let citation: Citation = match serde_json::from_str(cite_str) {
        Ok(c) => c,
        Err(_) => return ptr::null_mut(),
    };

    match processor.process_citation_with_format::<Html>(&citation) {
        Ok(rendered) => safe_c_string(rendered),
        Err(_) => ptr::null_mut(),
    }
}

/// Render a citation to a Plain Text string.
///
/// # Safety
/// The caller must ensure that `processor` is a valid pointer and
/// `cite_json` is a valid null-terminated C string. The returned
/// string must be freed with `csln_string_free`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn csln_render_citation_plain(
    processor: *mut Processor,
    cite_json: *const c_char,
) -> *mut c_char {
    if processor.is_null() || cite_json.is_null() {
        return ptr::null_mut();
    }

    let processor = unsafe { &*processor };
    let cite_str = match unsafe { CStr::from_ptr(cite_json) }.to_str() {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };

    let citation: Citation = match serde_json::from_str(cite_str) {
        Ok(c) => c,
        Err(_) => return ptr::null_mut(),
    };

    match processor.process_citation_with_format::<PlainText>(&citation) {
        Ok(rendered) => safe_c_string(rendered),
        Err(_) => ptr::null_mut(),
    }
}

/// Render the bibliography to a LaTeX string.
///
/// # Safety
/// The caller must ensure that `processor` is a valid pointer.
/// The returned string must be freed with `csln_string_free`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn csln_render_bibliography_latex(processor: *mut Processor) -> *mut c_char {
    if processor.is_null() {
        return ptr::null_mut();
    }

    let processor = unsafe { &*processor };
    let rendered = processor.render_bibliography_with_format::<Latex>();
    safe_c_string(rendered)
}

/// Render the bibliography to an HTML string.
///
/// # Safety
/// The caller must ensure that `processor` is a valid pointer.
/// The returned string must be freed with `csln_string_free`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn csln_render_bibliography_html(processor: *mut Processor) -> *mut c_char {
    if processor.is_null() {
        return ptr::null_mut();
    }

    let processor = unsafe { &*processor };
    let rendered = processor.render_bibliography_with_format::<Html>();
    safe_c_string(rendered)
}

/// Render the bibliography to a Plain Text string.
///
/// # Safety
/// The caller must ensure that `processor` is a valid pointer.
/// The returned string must be freed with `csln_string_free`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn csln_render_bibliography_plain(processor: *mut Processor) -> *mut c_char {
    if processor.is_null() {
        return ptr::null_mut();
    }

    let processor = unsafe { &*processor };
    let rendered = processor.render_bibliography_with_format::<PlainText>();
    safe_c_string(rendered)
}

/// Free a string allocated by the processor.
///
/// # Safety
/// The pointer must have been returned by one of the rendering functions.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn csln_string_free(s: *mut c_char) {
    if !s.is_null() {
        let _ = unsafe { CString::from_raw(s) };
    }
}
