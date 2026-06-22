# Changelog

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
