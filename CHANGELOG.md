# Changelog

## 0.3.1 - 2026-07-01

### Added

- Added sleek indeterminate loading stripes to the parsed and hex panes to indicate background data fetching.
- Added dynamic upper-bound index limits to the search input placeholders based on table size.

### Changed

- Improved the Parsed Data table selection styling specificity so the blue active selection highlight is never overwritten by passive search highlights.
- Updated the variation `deltaSets` parsing to conform perfectly to the OpenType spec (producing `DeltaSet` records with fully unpackaged, explicitly typed, and selectable `ParsedField` elements).
- Disabled spellchecking across all search inputs.
- Empty arrays now render correctly as `[]` instead of `[0 items]` or `[0 records]`.

### Fixed

- Fixed search iteration logic that was entirely skipping index `0`.
- Fixed the backend search logic which could get permanently stuck when finding the first match in massive lazy tables.
- Improved search highlighting to target specific matched cells rather than highlighting the entire row.
- Fixed the `Index` column search logic to enforce exact numeric matching instead of partial string matching.

## 0.3.0 - 2026-06-30

### Added

- Added `glyf` and `loca` table parsers with backend lazy-loading support for massive fonts.
- Added `vhea`, `vmtx`, `vvar` table parsers.
- Added `HVAR` and `MVAR` table parsers.

### Changed

- Massively improved Hex pane scroll rendering performance for huge tables.
- Improved Hex pane auto-scroll behavior to use nearest-edge snapping.
- Improved object value inline formatting by adding ellipsis truncation for large objects.

### Fixed

- Fixed `itemVariationData` selection bounds and simplified mapData representation.

## 0.2.0 - 2026-06-22

### Added

- Added structured `fvar` parser.
- Added structured `STAT` parser.
- Added structured `avar` parser.
- Added shared parser support for common OpenType variation structures.
- Added member type labels in nested parsed data views.

### Changed

- Improved OpenType numeric type labels for parsed fields.
- Split major/minor version fields where the table spec exposes them separately.

## 0.1.0 - 2026-06-22

### Added

- Initial release.
- Added structured parsers for Table Directory, `cmap`, `head`, `hhea`, `hmtx`, `maxp`, `name`, `OS/2`, and `post`.
- Added raw hex pane with parsed-field byte selection.
- Added Windows titlebar context menu.
