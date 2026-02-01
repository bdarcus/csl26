# Remaining APA Bibliography Issues

**Status**: 15/15 citations ✅, 11/15 bibliography

## Issues to Fix

### 1. Thesis Bracket Formatting
**Oracle**: `Neural Networks for Natural Language Understanding [PhD thesis]. Stanford University.`
**CSLN**: `Neural Networks for Natural Language Understanding. PhD thesis. Stanford University.`

**Problem**: Genre should be wrapped in brackets and attached to title (no period separator).

**Root Cause**: Migration architecture issue. The `description` macro in APA has `<group prefix="[" suffix="]">` containing a `<choose>` with type-specific branches. When thesis type is processed:
1. Genre variable is first seen in other branches without brackets
2. Later seen in thesis branch with brackets
3. Due to how `compile_with_wrap` creates local component lists per recursion, the thesis-specific bracket wrap is not captured as an override

**Fix Required**: Major refactor of template_compiler.rs to pass a shared mutable components reference through recursive calls, or post-process to detect and add type-specific wrap overrides.

**Complexity**: High

---

### 2. Edition Parentheses Formatting
**Oracle**: `The Psychology of Computer Programming (Silver Anniversary Edition). Van Nostrand Reinhold.`
**CSLN**: `The Psychology of Computer Programming. Silver Anniversary Edition. Van Nostrand Reinhold.`

**Problem**: Edition should be in parentheses and attached to title (no period separator).

**Root Cause**: Same issue as thesis - the edition is wrapped in parentheses in CSL 1.0 but the migration doesn't capture this as a type-specific override for books.

**Complexity**: High (same fix as thesis)

---

### 3. Proceedings Container-Pages Delimiter  
**Oracle**: `Proceedings of NIPS 2013, 3111–3119.`
**CSLN**: `Proceedings of NIPS 2013. 3111–3119.`

**Problem**: Pages should use comma delimiter after container-title, not period.

**Root Cause**: The volume-pages-delimiter config may not apply to paper-conference type, or the container-pages relationship isn't being detected correctly.

**Complexity**: Medium

---

### 4. Edited Book Author/Editor Rendering
**Oracle**: `Reis, H. T., & Judd, C. M. (Eds.). (2000). Handbook of Research Methods...`
**CSLN**: `Reis, H. T., & Judd, C. M. (2000). Handbook of Research Methods... In H. T. Reis, & C. M. Judd (Eds.),...`

**Problem**: For edited books where editors ARE the primary authors, the `(Eds.)` label should appear after their names before the date, not as a separate "In ... (Eds.)" construction.

**Root Cause**: The migration is treating editors as a secondary contributor group instead of recognizing when editors are the primary contributors for edited-book type.

**Complexity**: Medium-High

---

## Completed in This Session

- PR #56: Fix Chicago bibliography names (initials)
- PR #57: Fix bibliography sorting (multi-key, editor fallback)
- PR #58: Citation grouping and year suffix ordering
- PR #59: Strip leading articles + fix anonymous work formatting
- PR #60: Configurable URL trailing period (5/15 → 11/15)

## Next Steps

1. For thesis/edition: Consider adding a post-processing step in migration that detects common patterns and adds appropriate overrides
2. For proceedings delimiter: Check paper-conference type handling in delimiter logic
3. For edited books: Add logic to detect when editor IS the author and render differently
