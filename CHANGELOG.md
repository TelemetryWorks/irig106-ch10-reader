# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

Types of changes
- `Added` for new features.
- `Changed` for changes in existing functionality.
- `Deprecated` for soon-to-be removed features.
- `Removed` for now removed features.
- `Fixed` for any bug fixes.
- `Security` in case of vulnerabilities.

<!-- release-notes -->
## [Unreleased]

## [0.1.0] - 2026-03-22

### Added

- Initial `ch10r` Rust CLI for fast structural inspection of IRIG 106 Chapter 10 files.
- Memory-mapped scanning for large `.ch10` recordings with packet-by-packet traversal.
- Summary reporting for packet counts, channel inventory, data type distribution, and byte totals.
- Optional verbose packet listing with `--packets` and packet limiting with `--limit N`.
- Header checksum validation, sequence-gap detection, TMATS presence checks, and sync-recovery scanning.
- Windows-only executable support for the initial release.

### Changed

- Expanded the project README with usage examples, scope, supported data types, checksum behavior, and current limitations.
