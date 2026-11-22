# Reader features and test cases for irig106-core

Owner decision, 2026-11-22: archive this repository because packet reading belongs
in irig106-core and inspection belongs in the future `irig106 inspect` command.
This inventory records the reader at main `2fdc477`; it is requirements input,
not a claim that core or the CLI already implements these behaviors. Historical
Windows releases v0.1.0–v0.1.2 and their usage documentation remain available.

Evidence is source inspection of [src/lib.rs](../src/lib.rs),
[tests/parser.rs](../tests/parser.rs) and [src/lib_tests.rs](../src/lib_tests.rs).
The ten tests use synthetic headers/recordings; they do not constitute real-file,
corruption-recovery or full standards-conformance coverage. No tests were executed
to produce this documentation-only inventory.

## Implemented features to consider

| Area | Reader behavior and boundary |
| --- | --- |
| Packet reading | Memory-maps a file and follows declared packet lengths. Reads the 24-byte primary header, little-endian sync `0xEB25`, channel, lengths, sequence, flags, data type and 48-bit RTC. Payloads are not decoded. |
| Summaries | Reports file name/size, processed packets and unique channel IDs. Rows are keyed by **(channel ID, data type)**, preserving multiple data types on channel zero. Tracks packet/data byte totals, min/max payload length, latest RTC and error counts; the displayed table includes packet counts, data bytes and min/max lengths. Recording/start/stop times remain “Not available.” |
| Data-type counts | Each channel/type row carries a packet count and type/format/detail labels. Counts are not a separate global per-type aggregate. Labels include TMATS `0x01`, events `0x02`, index `0x03` and common telemetry formats; unknown/reserved values fall back to labels. |
| Header checksums | Sums the first eleven little-endian 16-bit primary-header words modulo 65536, comparing the stored checksum. `PacketHeader::parse` retains a sync-valid header even when its checksum is bad; `is_valid_header` rejects it. Ordinary traversal reports bad checksums; recovery candidates must pass them. This does not validate payload or secondary-header checksums. |
| Sequence gaps | Expects the previous sequence plus one with wrapping `u8` arithmetic. Counts discontinuities and records packet/channel/expected/actual detail. State is currently per channel/type row, not shared across a channel's data types; requirements should resolve that distinction rather than inherit it accidentally. |
| TMATS presence | Marks present on channel `0`, data type `0x01`; also reports index/event presence for types `0x03`/`0x02`. Does not parse TMATS, validate its contents, or ensure a required setup record is first. |
| Structural issues | Handles zero/sub-header packet lengths, packets extending beyond EOF and payload lengths larger than packet space. Records offsets and other detail; prints at most 20 structural issues. Empty/sub-header files are rejected before mapping. |
| Inspection options | `--packets` prints headers after the summary; `--limit N` stops after N counted packets. Help/version expose Cargo metadata. These are historical CLI behaviors, not a prescribed core API. |

## Sync recovery limits and edge cases

On a missing sync word, the reader searches byte-by-byte from `offset + 1` toward
`min(file_length, offset + 1,048,576)`. A candidate must have a sync word, valid
primary checksum, packet length at least 24, and an entire declared packet inside
the file. Recovery reports skipped bytes; failure reports the scanned window and
stops. It does not prove payload validity or a complete valid packet chain.

The search is an exclusive Rust range ending at `search_end.saturating_sub(24)`;
a header starting exactly at that boundary is **not inspected**. Zero/sub-header
lengths advance one byte before re-entering traversal/recovery. Fewer than 24
trailing bytes stop traversal without a trailing-fragment issue. The truncated
packet branch counts the header in row statistics, then exits before the usual
global checksum and TMATS/index/event updates. These are source observations to
review when specifying core, not tested guarantees or recommended semantics.
Recovery has a bounded scan window, but diagnostic vectors and verbose rows can
grow with the input; the reader is not a general bounded-memory streaming API.

## Existing test coverage

Names below are exact test functions in the linked files. “Covered” means source
assertions exist, not that a check was executed during this handoff.

| Test | Covered cases/assertions |
| --- | --- |
| `computes_and_validates_header_checksum` | Generated primary-header checksum equals the stored value and the header validates. |
| `rejects_header_with_bad_checksum_even_when_sync_matches` | Corrupt stored checksum is rejected by `is_valid_header`; `parse` still returns fields with `checksum_valid=false`. |
| `parses_packet_header_fields_and_flags` | Channel/length/sequence/type/48-bit RTC fields; secondary-header flag, 8-bit checksum-type flag, overflow and RTC-sync-error flags. It does not parse a secondary header. |
| `parse_returns_none_for_short_or_non_sync_buffers` | 23-byte input and wrong sync word return `None`. |
| `channel_stats_tracks_gaps_checksums_and_flags` | Two packets with sequence 7 then 9; one gap and detail text, bad checksum, overflow/sync flags, packet/data byte sums, min/max data length and last sequence/RTC. |
| `cli_keeps_same_channel_id_with_different_data_types_separate` | Synthetic channel-zero TMATS/index plus a second channel's event packet: two unique channels, separate rows, and positive TMATS/index/event status. |
| `cli_reports_indexing_not_found_when_no_index_packets_exist` | One time packet gives absent TMATS, index and event status. |
| `cli_help_shows_options_and_cargo_metadata` | Successful `--help`, empty stderr, usage/options and package/binary/version metadata. |
| `cli_version_shows_version_and_metadata` | Successful `--version`, empty stderr and package/binary/version/description output. |
| `data_type_labels_match_common_chapter10_definitions` | Representative computer-generated, PCM reserved/format-1, time, video, image, IEEE 1394, TSPI, CAN and Fibre Channel labels. Not an exhaustive mapping/conformance test. |

## Requirements and coverage gaps for the successor

- Add recovery tests for a valid candidate, false sync/bad checksum, short or
  overrunning candidate, no candidate, exact scan-window/end-of-file boundary,
  zero/sub-header lengths and trailing fragments. Existing tests exercise none
  of the CLI recovery paths or the 1 MiB limit.
- Specify per-channel versus per-channel/type sequence state. Cover wraparound,
  repeated/out-of-order sequences and interleaved data types; the existing gap
  test only exercises 7 to 9 within one row.
- Cover malformed data lengths and truncated packets, including consistent
  counts/checksum/presence reporting; also empty files, unreadable paths and
  `--packets`/`--limit` behavior. Those paths currently lack regression assertions.
- Decide whether inspection needs global type totals and setup-record ordering
  checks. Existing row counts and a presence boolean do not establish either.
- Treat [ROADMAP.md](ROADMAP.md)'s details mode, width/layout controls, packet
  filtering/selection, field-oriented hex and hexadecimal offsets as **planned**,
  not implemented features or tests. Any successor adoption needs its own
  requirements and validation; no implementation is copied by this PR.
