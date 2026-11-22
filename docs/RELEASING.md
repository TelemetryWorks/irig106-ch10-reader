# Releasing the reader

This adopts the [owner-approved org standard at fcceb02](https://github.com/TelemetryWorks/.github/blob/fcceb02/docs/TRUSTED-PUBLISHING.md),
binary variant, in `trusted-publishing.yml` (**Trusted publishing**). Adoption is
not release permission. No version/MSRV/dependency/platform change is made.

## Build and signing boundary

The read-only `build-binaries-core.yml` builds/tests **every** binary target on
Windows x86_64 MSVC, archives the executables with LICENSE and README, and
smoke-tests the ZIP outside the checkout. It also runs release-control tests,
`cargo package --locked` with packaged-source compilation, and generates a
source-graph SBOM with locked `cargo-cyclonedx` 0.5.9. Ubuntu packaging is a build
check, not Linux support. Exact Rust 1.99.1 is recorded, logged and reused for
repackaging and registry installation; MSRV remains 1.85. Worker/thread counts
are capped at two; jobs have timeouts.

The immutable `release-<version>` payload contains the `.crate`,
`ch10r-v<version>-windows-x86_64.zip`, `sbom.cdx.json` and one `SHA256SUMS`.
`build-binaries.yml` invokes the core and signs only **its own fixed output** in
a nested **Checksums and attestations** job. Its independent tag-push guard and
`github.sha` checkout do not trust a caller boolean alone. All three payload
files are attested before registry approval. A separate immutable artifact
`release-<version>-attestations` contains the single
`irig106-ch10-reader-<version>.intoto.jsonl` bundle. Together they form one logical
bundle; no upload overwrites the payload. Source-graph SBOM/provenance is not a
claim of complete native dependencies or an achieved SLSA level.

PR and manual runs (even manual dispatch from a tag) are unsigned and read-only,
with no OIDC or approval environment. **Publish path (dry run)** validates all
checksums, reproduces the crate without compiling using the build's exact Rust,
and compares bytes. It also checks a committed digest-pinned types 0.1.0 registry
fixture and all historical reader assets. Dry-run comparisons are not signatures.

**Standard adjustment:** reusable caller permissions cannot be conditional.
Following spatial-typestate #42, tag-only `build` delegates to the signer wrapper;
`build-dry-run` delegates to the read-only core. The dry-run job therefore needs
`[gate, build-dry-run]`, instead of `[gate, build]`. This ensures no running PR
job holds `id-token: write`. A separate native Windows verifier supplements the
Ubuntu `verify` job to actually execute published Windows binaries.

## OWNER FIRST-UPLOAD BOOTSTRAP REQUIRED

crates.io returned 404 for the reader on 2026-11-22. Tag runs refuse publication
until the crate exists and `crate.trustpub_only == true`; PR/manual runs report
this blocker and continue with `publish=false`. CI never bootstraps or stores a
token. The [2026-11-15 owner decision](https://github.com/TelemetryWorks/.github/blob/fcceb02/SECURITY-PROGRAM.md#publishing-policy)
allows only the owner/coordinator's first upload using a one-day, crate-scoped
`publish-new` token, immediately revoked afterward. Adoption authorizes none of
that credential handling or publication.

Owner checklist before any new authorized tag; **not certified by this PR**:

- Complete the one-time bootstrap, revoke its token, register crates.io Trusted
  Publisher `TelemetryWorks/irig106-ch10-reader`, filename `trusted-publishing.yml`,
  environment `crates-io`, and enable Trusted Publishing only. Verify the public
  `crate.trustpub_only` flag and private publisher binding separately.
- Protect `crates-io` with owner-required approval, no administrator bypass and
  eligibility restricted to `v*` **tags**, not branches. YAML naming is not proof.
- Prepare a new version beyond v0.1.0–v0.1.2, a dated
  `## [X.Y.Z] - YYYY-MM-DD` changelog entry, and committed
  `docs/releases/vX.Y.Z.md`: first heading is title; remaining body must equal
  the changelog entry. Historical notes are not fabricated by adoption.
- Require successful **Windows CI** push-to-main for the exact release commit.
  Only the owner may then authorize an annotated `vX.Y.Z` tag reachable from main.

The gate checks remote annotation via GitHub ref/tag-object APIs, peels to the
immutable event/checkout commit, checks ancestry and all intended CI run pages,
and requires dated committed notes. Missing tag evidence fails before build.
Every privileged job also independently checks the actual tag-push event.

Only `publish` enters `crates-io`. It verifies checksums and signatures bound to
repo, reusable `build-binaries.yml` signer, tag and commit; repackages with
`--no-verify`; compares bytes; obtains a fresh OIDC token only for a new version;
and publishes with `--locked --no-verify`. Nothing compiles under publication
credentials. Existing/new registry versions must match served bytes and checksum.

`github-release` lists and resumes drafts deliberately. It checks tag, title,
exact committed body, draft/pre-release state, all asset names/sizes/digests
before publishing, then checks again. Missing assets may be added; mismatches
fail. Matching published releases are read-only, never patched. Latest is
explicit: highest stable SemVer, excluding older versions, drafts/pre-releases.

`verify` downloads actual attached files, compares them to frozen evidence,
verifies signatures/registry identity and requires bounded docs.rs success;
pending fails. `verify-windows` verifies/smoke-tests the published ZIP, then
clean-installs the exact registry version into an empty root and checks version.
These privileged paths cannot be exercised by a PR dry run.

## Historical layouts stay valid

```sh
python3 scripts/release/pipeline.py legacy
python3 scripts/release/pipeline.py legacy --version 0.1.2
```

v0.1.0–v0.1.2 remain unsigned and unchanged. The legacy verifier accepts only the
exact ZIP and `.sha256` names/bytes pinned in `fixtures/legacy-releases.json`.
v0.1.0's digest text is uppercase. v0.1.2 has the correct archive digest but a
stale v0.1.1 filename and extra blank line; the verifier reports the anomaly,
without rewriting it or applying the future layout. No historical signatures,
crates.io or docs.rs successes are claimed. Old releases cannot enter publishing.

## Read-only checks

```sh
python3 -m unittest discover -s scripts/release -p 'test_*.py' -v
python3 scripts/release/pipeline.py known-good
python3 scripts/release/pipeline.py legacy
actionlint .github/workflows/*.yml  # 1.7.12
zizmor .github/workflows          # 1.33.0
```

Controls adapt Apache-2.0 types #20/#21; permission split follows spatial #41/#42.
See [adoption evidence](evals/2026-11-22-trusted-publishing.md). This workflow is
the sole future release path: current `windows-ci.yml` already has no tag/release
path to retire and is preserved as CI only. No tags, settings or releases changed.
