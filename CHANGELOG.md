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

## [0.1.2] - 2026-03-24

### Added

- Added first-class `-h` / `--help` and `-V` / `--version` CLI switches. Version output now comes from Cargo package metadata and includes the package name, binary name, and package description.

### Changed

- Expanded the help text to show the current package metadata and the supported CLI options in one place.
- Added integration tests covering the release-facing help and version output.
- Added rustdoc-style crate, module, and public API comments across the source to make the release surface easier to navigate and maintain.

## [0.1.1] - 2026-03-23

### Fixed

- Split summary rows by both channel ID and data type so channel `0` TMATS and index packets are reported separately instead of being merged. ([`a2163c1`](https://github.com/TelemetryWorks/irig106-ch10-reader/commit/a2163c1) by [@joey-huckabee](https://github.com/joey-huckabee))
- Corrected the computer-generated data type labels so TMATS is recognized as `0x01` and recording index data as `0x03`. ([`a2163c1`](https://github.com/TelemetryWorks/irig106-ch10-reader/commit/a2163c1) by [@joey-huckabee](https://github.com/joey-huckabee))
- Corrected additional Chapter 10 data type mappings so PCM `0x09` reports as `Format 1` and later video, image, Ethernet, IEEE 1394, TSPI/CTS, and Fibre Channel labels align with the IRIG type table. ([`bb843a6`](https://github.com/TelemetryWorks/irig106-ch10-reader/commit/bb843a6) by [@joey-huckabee](https://github.com/joey-huckabee))
- Added CLI regression tests covering multiple channel-zero packet types and indexing/event summary detection. ([`a2163c1`](https://github.com/TelemetryWorks/irig106-ch10-reader/commit/a2163c1), [`9fcf292`](https://github.com/TelemetryWorks/irig106-ch10-reader/commit/9fcf292), [`dc798e6`](https://github.com/TelemetryWorks/irig106-ch10-reader/commit/dc798e6) by [@joey-huckabee](https://github.com/joey-huckabee))
- Added an `Indexing` summary line that reports `Enabled` when recording index packets (`0x03`) are present. ([`9fcf292`](https://github.com/TelemetryWorks/irig106-ch10-reader/commit/9fcf292) by [@joey-huckabee](https://github.com/joey-huckabee))
- Added an `Events` summary line that reports `Enabled` when recording event packets (`0x02`) are present. ([`dc798e6`](https://github.com/TelemetryWorks/irig106-ch10-reader/commit/dc798e6) by [@joey-huckabee](https://github.com/joey-huckabee))

### Changed

- Moved the standards mapping assertions into a dedicated `src/lib_tests.rs` test module so production code and test code are clearly separated. ([`bb843a6`](https://github.com/TelemetryWorks/irig106-ch10-reader/commit/bb843a6) by [@joey-huckabee](https://github.com/joey-huckabee))

## [0.1.0] - 2026-03-22

### Added

- Initial `ch10r` Rust CLI for fast structural inspection of IRIG 106 Chapter 10 files. ([`298b770`](https://github.com/TelemetryWorks/irig106-ch10-reader/commit/298b7708882470b0a0392ed171ddba9deea54610) by [@joey-huckabee](https://github.com/joey-huckabee))
- Memory-mapped scanning for large `.ch10` recordings with packet-by-packet traversal. ([`298b770`](https://github.com/TelemetryWorks/irig106-ch10-reader/commit/298b7708882470b0a0392ed171ddba9deea54610) by [@joey-huckabee](https://github.com/joey-huckabee))
- Summary reporting for packet counts, channel inventory, data type distribution, and byte totals. ([`298b770`](https://github.com/TelemetryWorks/irig106-ch10-reader/commit/298b7708882470b0a0392ed171ddba9deea54610) by [@joey-huckabee](https://github.com/joey-huckabee))
- Optional verbose packet listing with `--packets` and packet limiting with `--limit N`. ([`298b770`](https://github.com/TelemetryWorks/irig106-ch10-reader/commit/298b7708882470b0a0392ed171ddba9deea54610) by [@joey-huckabee](https://github.com/joey-huckabee))
- Header checksum validation, sequence-gap detection, TMATS presence checks, and sync-recovery scanning. ([`298b770`](https://github.com/TelemetryWorks/irig106-ch10-reader/commit/298b7708882470b0a0392ed171ddba9deea54610) by [@joey-huckabee](https://github.com/joey-huckabee))
- Windows-only executable support for the initial release. ([`298b770`](https://github.com/TelemetryWorks/irig106-ch10-reader/commit/298b7708882470b0a0392ed171ddba9deea54610) by [@joey-huckabee](https://github.com/joey-huckabee))
- Added platform-specific documentation under `docs/`, including a Windows usage guide and placeholder Linux/macOS guides. ([`2f16483`](https://github.com/TelemetryWorks/irig106-ch10-reader/commit/2f16483136290704bde454ebd0f29b6be5f9b601) by [@joey-huckabee](https://github.com/joey-huckabee))
- Added a simple `CONTRIBUTING.md` with project scope, development commands, and pull request expectations. ([`2f16483`](https://github.com/TelemetryWorks/irig106-ch10-reader/commit/2f16483136290704bde454ebd0f29b6be5f9b601) by [@joey-huckabee](https://github.com/joey-huckabee))
- Added parser-focused unit tests covering checksum validation, header parsing, and sequence-gap/error tracking. ([`792a9c2`](https://github.com/TelemetryWorks/irig106-ch10-reader/commit/792a9c29bc31256d0a098f1dbb3b91b8dd076678) by [@joey-huckabee](https://github.com/joey-huckabee))
- Added `docs/ROADMAP.md` to capture future detail-mode, TMATS-output, and width-aware output direction. ([`da3fc84`](https://github.com/TelemetryWorks/irig106-ch10-reader/commit/da3fc845cf15f1ac6b7c8434202bd7d5f9450fca) by [@joey-huckabee](https://github.com/joey-huckabee))

### Changed

- Expanded the project README with usage examples, scope, supported data types, checksum behavior, and current limitations. ([`d30a32f`](https://github.com/TelemetryWorks/irig106-ch10-reader/commit/d30a32f534ae3c862170a0b189a895bd64e15f54) by [@joey-huckabee](https://github.com/joey-huckabee))
- Reworked the top-level README to link to operating-system-specific documentation and clarify that Windows is the only supported `0.1.0` target. ([`2f16483`](https://github.com/TelemetryWorks/irig106-ch10-reader/commit/2f16483136290704bde454ebd0f29b6be5f9b601) by [@joey-huckabee](https://github.com/joey-huckabee))
- Redesigned the default CLI output into a left-aligned summary plus a single consolidated channel/data-type table. ([`da3fc84`](https://github.com/TelemetryWorks/irig106-ch10-reader/commit/da3fc845cf15f1ac6b7c8434202bd7d5f9450fca) by [@joey-huckabee](https://github.com/joey-huckabee))
- Moved verbose packet listing to the end of the output after the default summary. ([`da3fc84`](https://github.com/TelemetryWorks/irig106-ch10-reader/commit/da3fc845cf15f1ac6b7c8434202bd7d5f9450fca) by [@joey-huckabee](https://github.com/joey-huckabee))
- Split the channel table description into aligned `Data Type`, `Data Format`, and `Data Detail` columns for clearer reading. ([`c54a880`](https://github.com/TelemetryWorks/irig106-ch10-reader/commit/c54a880a7037b56e6f2fbd80fe72d8eaf0d6bb2d) by [@joey-huckabee](https://github.com/joey-huckabee))

### Fixed

- Corrected the CLI usage text to reference the actual `ch10r` executable name. ([`6282abf`](https://github.com/TelemetryWorks/irig106-ch10-reader/commit/6282abf3a988629e8dad054758ed8f6ff10ca7b3) by [@joey-huckabee](https://github.com/joey-huckabee))
- Replaced the Unix-specific TMATS extraction command in CLI output with Windows-oriented guidance. ([`2f16483`](https://github.com/TelemetryWorks/irig106-ch10-reader/commit/2f16483136290704bde454ebd0f29b6be5f9b601) by [@joey-huckabee](https://github.com/joey-huckabee))
- Replaced Unicode-heavy and garbled terminal output with ASCII-safe formatting for Windows console compatibility. ([`896c9eb`](https://github.com/TelemetryWorks/irig106-ch10-reader/commit/896c9eb279ac6652e56a89d04ee161d7f71819f1) by [@joey-huckabee](https://github.com/joey-huckabee))
- Removed the verbose TMATS warning block from the default output while keeping a simple TMATS status line. ([`da3fc84`](https://github.com/TelemetryWorks/irig106-ch10-reader/commit/da3fc845cf15f1ac6b7c8434202bd7d5f9450fca) by [@joey-huckabee](https://github.com/joey-huckabee))
