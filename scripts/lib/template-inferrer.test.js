const { detectNameOrder } = require('./template-inferrer');
const { strict: assert } = require('assert');

// Mock helpers
function makeName(family, given) {
    return [{ family, given }];
}

console.log('Running detectNameOrder tests...');

// Basic cases
assert.equal(detectNameOrder('Smith, John', makeName('Smith', 'John')), 'family-first', 'Basic family-first failed');
assert.equal(detectNameOrder('John Smith', makeName('Smith', 'John')), 'given-first', 'Basic given-first failed');
assert.equal(detectNameOrder('Smith, J.', makeName('Smith', 'John')), 'family-first', 'Initials family-first failed');
assert.equal(detectNameOrder('J. Smith', makeName('Smith', 'John')), 'given-first', 'Initials given-first failed');

// Initials without periods (boundary checks)
assert.equal(detectNameOrder('Smith J', makeName('Smith', 'John')), 'family-first', 'No-period initial family-first failed');
assert.equal(detectNameOrder('J Smith', makeName('Smith', 'John')), 'given-first', 'No-period initial given-first failed');

// Case insensitivity
assert.equal(detectNameOrder('smith, john', makeName('Smith', 'John')), 'family-first', 'Case insensitive failed');

// Literal names (should fail gracefully)
assert.equal(detectNameOrder('World Bank', [{ literal: 'World Bank' }]), null, 'Literal name should allow null');

// Missing parts
assert.equal(detectNameOrder('Smith', makeName('Smith', '')), null, 'Missing given name should be null');

// Initial matching window
// "Thomas, B." where given is "Brian"
assert.equal(detectNameOrder('Thomas, B.', makeName('Thomas', 'Brian')), 'family-first', 'Initial matching failed');

// Complex text (simulating window extraction)
const text1 = 'In K. A. Ericsson, N. Charness';
const names1 = makeName('Ericsson', 'K. Anders');
assert.equal(detectNameOrder(text1, names1), 'given-first', 'Complex editor window failed');

const text2 = 'Ericsson, K. A., Charness, N.';
assert.equal(detectNameOrder(text2, names1), 'family-first', 'Complex author window failed');

console.log('All detectNameOrder tests passed!');
