'use strict';

/**
 * Return citeproc bibliography IDs aligned with the rows it actually rendered.
 *
 * citeproc-js records an `entry_ids` slot before rendering each bibliography
 * item, but omits the corresponding string when the item has no rendered form.
 * Its zero-based `bibliography_errors[].index` identifies those failed slots.
 */
function bibliographyIdsForRenderedRows(params, bibliography) {
  const entryIds = Array.isArray(params?.entry_ids)
    ? params.entry_ids.map((ids) => ids?.[0] ?? null)
    : [];

  if (entryIds.length === bibliography.length) return entryIds;

  const failedIndexes = new Set(
    (Array.isArray(params?.bibliography_errors) ? params.bibliography_errors : [])
      .map((error) => error?.index)
      .filter((index) => Number.isInteger(index) && index >= 0 && index < entryIds.length)
  );

  return entryIds.filter((_id, index) => !failedIndexes.has(index));
}

module.exports = { bibliographyIdsForRenderedRows };
