# Reader release-pipeline adoption — 2026-11-22

Baseline: `2fdc4773ff304169e2942a4e9934e30f32e93dd7`.
Origin: https://github.com/TelemetryWorks/irig106-ch10-reader.git.
Read the entire org standard `fcceb02`, types #20/#21 and spatial #41/#42.

Read-only real-world evidence before implementation:

- Current windows-ci.yml is CI only: no tag trigger or release upload/job to retire.
- Reader crates.io endpoint returns HTTP 404; no trustpub_only value exists yet.
  No private publisher/environment protection is certified or changed.
- All three GitHub Releases and all six assets inspected/hashed. ZIPs contain
  exactly ch10r.exe, LICENSE, README. Historical digest pins preserve uppercase
  v0.1.0 and the stale filename/blank line in v0.1.2. Its digest is correct.
- Independent known-good fixture: types 0.1.0 from reviewed types #21, SHA-256
  2f5ddd5baf4fbc4fa5579bd739561274bc5fc9cc36ba50d610b0442cc0635420.
- Intended CI selection uses exact workflow/event/branch/SHA/conclusion and all
  API pages. No status=success API filter; unrelated green jobs do not qualify.
- Pinned action contracts inspected from adopted workflows. Local actionlint
  1.7.12 and zizmor 1.33.0 binaries verified against GitHub release-asset digests.

Release-control tests cover corruption/missing/extra/symlink/duplicate/traversal
files, archive members/all binary targets, remote annotation, CI identity,
bootstrap refusal, unsigned PR/manual, dated/committed notes, exact repackaging,
served registry comparison, signature mutation barriers, drafts, immutable
published assets, explicit Latest, docs.rs fail-on-pending, historical layouts
and permissions/immutable checkouts. Live known-good and all legacy comparisons
exercise real read-only download paths. No publication is used as a test.

Standard feedback: reusable caller write permissions cannot vary by event.
Use spatial #42's separate read-only dry-run caller, with its matching needs.
Immutable payload and sidecar are transported together, never overwritten.
Windows smoke verification supplements Ubuntu verification. No SLSA-level or
native dependency completeness claim is made.

Required PR dry-run run ID/results are recorded in the adoption PR after execution.
Local checks are not a substitute. Signing, approvals, registry/GitHub publication
and docs.rs polling are not exercised by PRs. Owner checklist in RELEASING.md is
still required before any next authorized tag. Historical releases stay intact.
