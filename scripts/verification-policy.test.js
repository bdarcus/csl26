const test = require('node:test');
const assert = require('node:assert/strict');

const { resolveScopeAuthority } = require('./lib/verification-policy');

// resolveScopeAuthority() operates on an already-resolved style policy
// object, so it is tested against a synthetic fixture rather than a real
// style's verification-policy.yaml entry. This used to be exercised via
// chem-acs (removed when the compound-chemistry styles relocated to
// citum-styles; see docs/architecture/audits/ for the split rationale) but
// the resolver is generic library logic that should not be anchored to a
// style that can relocate between repos.
test('resolveScopeAuthority falls back to the style-level authority when a scope has no override', () => {
  const stylePolicy = {
    authority: 'biblatex',
    authorityId: 'example-style',
    note: 'Bibliography authority follows biblatex; citation authority remains citeproc-js.',
    scopeAuthorities: {
      citation: {
        authority: 'citeproc-js',
      },
    },
  };

  const citationAuthority = resolveScopeAuthority(stylePolicy, 'citation');
  const bibliographyAuthority = resolveScopeAuthority(stylePolicy, 'bibliography');

  assert.deepEqual(citationAuthority, {
    authority: 'citeproc-js',
    authorityId: null,
    note: stylePolicy.note,
  });
  assert.deepEqual(bibliographyAuthority, {
    authority: 'biblatex',
    authorityId: 'example-style',
    note: stylePolicy.note,
  });
});

test('resolveScopeAuthority preserves an explicit scope-level authority_id override', () => {
  const stylePolicy = {
    authority: 'biblatex',
    authorityId: 'example-style',
    note: null,
    scopeAuthorities: {
      citation: {
        authority: 'citeproc-js',
        authority_id: 'example-style-citation-variant',
      },
    },
  };

  const citationAuthority = resolveScopeAuthority(stylePolicy, 'citation');

  assert.deepEqual(citationAuthority, {
    authority: 'citeproc-js',
    authorityId: 'example-style-citation-variant',
    note: null,
  });
});
