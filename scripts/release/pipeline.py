#!/usr/bin/env python3
"""Single-crate release controls. Dry-run commands never authenticate or publish."""

import argparse
import datetime
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import time
import tomllib
import tempfile
import zipfile
import urllib.error
import urllib.parse
import urllib.request

NAME = "irig106-ch10-reader"
KNOWN_NAME = "irig106-types"
REPO = "TelemetryWorks/irig106-ch10-reader"
WORKFLOW = f"{REPO}/.github/workflows/build-binaries.yml"
KNOWN_VERSION = "0.1.0"
KNOWN_SHA256 = "2f5ddd5baf4fbc4fa5579bd739561274bc5fc9cc36ba50d610b0442cc0635420"
USER_AGENT = f"TelemetryWorks-release-pilot (https://github.com/{REPO})"


def run(*args, **kwargs):
    return subprocess.check_output(args, text=True, **kwargs).strip()


def gh_api(path):
    return json.loads(run("gh", "api", path))


def output(**values):
    for key, value in values.items():
        print(f"{key}={value}")
    if os.environ.get("GITHUB_OUTPUT"):
        with open(os.environ["GITHUB_OUTPUT"], "a") as stream:
            for key, value in values.items():
                stream.write(f"{key}={value}\n")


def summary(message):
    print(message)
    if os.environ.get("GITHUB_STEP_SUMMARY"):
        with open(os.environ["GITHUB_STEP_SUMMARY"], "a") as stream:
            stream.write(message + "\n")


def version():
    data = tomllib.loads(Path("Cargo.toml").read_text())
    package = data["package"]
    if "workspace" in data or package["name"] != NAME or package.get("publish") is False:
        raise ValueError("Single-crate pilot: review release-set ordering before changing workspace/publish metadata")
    value = package["version"]
    if not re.fullmatch(r"\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?", value):
        raise ValueError("Invalid package version")
    return value


def release_record(value):
    text = Path("CHANGELOG.md").read_text()
    match = re.search(rf"^## \[{re.escape(value)}\] - (\d{{4}}-\d{{2}}-\d{{2}})\s*$", text, re.M)
    if not match:
        raise ValueError(f"Missing dated CHANGELOG entry for {value}")
    datetime.date.fromisoformat(match[1])
    body = re.split(r"^## ", text[match.end():], maxsplit=1, flags=re.M)[0].strip()
    # Link definitions at the end of the changelog are not release notes.
    body = re.split(r"^\[[^\]]+\]:", body, maxsplit=1, flags=re.M)[0].strip()
    if not body:
        raise ValueError("Empty release changelog entry")
    path = Path(f"docs/releases/v{value}.md")
    notes = path.read_text()
    title, _, notes_body = notes.partition("\n")
    if not title.startswith("# ") or not title[2:].strip() or notes_body.strip() != body:
        raise ValueError("Release notes need a first heading and the exact CHANGELOG entry body")
    if run("git", "show", f"HEAD:{path}") != notes.strip():
        raise ValueError("Release notes must be committed at the release commit")
    return path, title[2:].strip()


def ci_succeeded(pages, commit):
    return any(
        item.get("head_sha") == commit
        and item.get("path") == ".github/workflows/windows-ci.yml"
        and item.get("conclusion") == "success"
        and item.get("event") == "push"
        and item.get("head_branch") == "main"
        for page in pages for item in page["workflow_runs"]
    )


def tag_commit(tag):
    obj = gh_api(f"repos/{REPO}/git/ref/tags/{tag}")["object"]
    if obj["type"] != "tag":
        raise ValueError("Remote release tag must be annotated")
    for _ in range(8):
        if obj["type"] != "tag":
            break
        obj = gh_api(f"repos/{REPO}/git/tags/{obj['sha']}")["object"]
    if obj["type"] != "commit":
        raise ValueError("Tag does not resolve to a commit")
    return obj["sha"]


def gate():
    value = version()
    commit = run("git", "rev-parse", "HEAD")
    event = os.environ["GITHUB_EVENT_NAME"]
    publishing = event == "push" and os.environ.get("GITHUB_REF", "").startswith("refs/tags/v")
    if event not in ("push", "pull_request", "workflow_dispatch") or (event == "push" and not publishing):
        raise ValueError("Unexpected release event")
    findings = []
    try:
        release_record(value)
    except (ValueError, FileNotFoundError, subprocess.CalledProcessError) as error:
        findings.append(str(error))
    raw = http_get(f"https://crates.io/api/v1/crates/{NAME}")
    if raw is None:
        findings.append("Crate is unpublished: owner must perform the 2026-11-15 FIRST-upload bootstrap, "
                        "revoke the one-day publish-new token, register trusted-publishing.yml / crates-io, "
                        "and enable Trusted Publishing only. CI never bootstraps. See docs/RELEASING.md.")
    elif json.loads(raw)["crate"].get("trustpub_only") is not True:
        findings.append("crates.io Trusted Publishing only must be enabled before release")
    if publishing:
        if value in legacy_records():
            findings.append("Historical releases cannot be re-released by this pipeline")
        tag = os.environ["GITHUB_REF_NAME"]
        if tag != f"v{value}":
            findings.append("Tag must exactly match the manifest version")
        else:
            if run("git", "cat-file", "-t", f"refs/tags/{tag}") != "tag":
                findings.append("Release tag must be annotated")
            if run("git", "rev-parse", f"refs/tags/{tag}^{{commit}}") != commit:
                findings.append("Checkout must equal the tagged commit")
            try:
                if tag_commit(tag) != commit:
                    findings.append("Remote annotated tag does not match the release commit")
            except ValueError as error:
                findings.append(str(error))
        if run("git", "rev-parse", os.environ["GITHUB_SHA"] + "^{commit}") != commit:
            findings.append("Checkout differs from the event commit")
        if subprocess.run(["git", "merge-base", "--is-ancestor", commit, "origin/main"]).returncode:
            findings.append("Release commit is not on main")
        pages = json.loads(run(
            "gh", "api", "--paginate", "--slurp",
            f"repos/{REPO}/actions/workflows/windows-ci.yml/runs?head_sha={commit}&per_page=100",
        ))
        if not ci_succeeded(pages, commit):
            findings.append("No successful push-to-main CI workflow run for this exact commit")
    else:
        summary("Dry run: publish=false; tag/ancestry/CI requirements are enforced only on tag pushes.")
    for finding in findings:
        summary(f"- Release gate finding: {finding}")
    if publishing and findings:
        raise ValueError("Release gate rejected the tag before build")
    output(version=value, commit=commit, publish=str(publishing).lower(),
           crates=json.dumps([{"name": NAME, "version": value, "manifest": "Cargo.toml"}], separators=(",", ":")))


def expected_files(value):
    return {f"{NAME}-{value}.crate", "sbom.cdx.json", archive_name(value)}


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def validate_bundle(directory, value):
    root = Path(directory)
    manifest = root / "SHA256SUMS"
    if manifest.is_symlink():
        raise ValueError("Symlink checksum manifest")
    checksums = {}
    for line in manifest.read_text().splitlines():
        match = re.fullmatch(r"([0-9a-f]{64})  ([A-Za-z0-9_.+-]+)", line)
        if not match or match[2] in checksums:
            raise ValueError("Malformed, unsafe or duplicate checksum entry")
        checksums[match[2]] = match[1]
    if set(checksums) != expected_files(value):
        raise ValueError("Checksum manifest must cover exactly the crate, SBOM and Windows archive")
    for name, checksum in checksums.items():
        path = root / name
        if path.is_symlink() or not path.is_file() or digest(path) != checksum:
            raise ValueError(f"Checksum/file mismatch: {name}")
    allowed = expected_files(value) | {"SHA256SUMS", f"{NAME}-{value}.intoto.jsonl"}
    if {p.name for p in root.iterdir()} - allowed:
        raise ValueError("Unexpected release artifact files")
    return checksums


def http_get(url):
    request = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
    for attempt in range(4):
        try:
            with urllib.request.urlopen(request, timeout=60) as response:
                return response.read()
        except urllib.error.HTTPError as error:
            if error.code == 404:
                return None
            if error.code != 429 and error.code < 500:
                raise
            if attempt == 3:
                raise
        except urllib.error.URLError:
            if attempt == 3:
                raise
        time.sleep(2)
    raise RuntimeError("HTTP retry exhausted")


def registry_metadata(value):
    raw = http_get(f"https://crates.io/api/v1/crates/{NAME}/{value}")
    if raw is None:
        return None
    data = json.loads(raw)["version"]
    if data["num"] != value or not re.fullmatch(r"[0-9a-f]{64}", data["checksum"]):
        raise ValueError("Invalid registry metadata")
    return data


def registry_compare(value, archive):
    metadata = registry_metadata(value)
    if metadata is None:
        raise ValueError(f"Registry version not available: {value}")
    raw = http_get(f"https://crates.io/api/v1/crates/{NAME}/{value}/download")
    if raw is None or hashlib.sha256(raw).hexdigest() != metadata["checksum"]:
        raise ValueError("Registry archive does not match registry checksum")
    if raw != Path(archive).read_bytes():
        raise ValueError("Registry archive is not byte-identical to the frozen crate")
    summary(f"Registry byte identity and checksum passed: {NAME} {value}, sha256:{metadata['checksum']}")


def check_rust(toolchain):
    if not re.fullmatch(r"\d+\.\d+\.\d+", toolchain):
        raise ValueError("Expected exact Rust toolchain version")
    actual = run("rustc", f"+{toolchain}", "--version").split()[1]
    if actual != toolchain:
        raise ValueError(f"Expected Rust {toolchain}, got {actual}")


def repackage(directory, value, toolchain):
    validate_bundle(directory, value)
    check_rust(toolchain)
    subprocess.run(["cargo", f"+{toolchain}", "package", "--locked", "--no-verify"], check=True)
    candidate = Path(f"target/package/{NAME}-{value}.crate")
    if candidate.read_bytes() != (Path(directory) / candidate.name).read_bytes():
        raise ValueError("Repackaged crate differs from frozen build artifact")
    summary(f"Non-compiling candidate byte comparison passed with Rust {toolchain}")


def known_good():
    fixture = Path(__file__).parent / "fixtures" / f"{KNOWN_NAME}-{KNOWN_VERSION}.crate"
    if digest(fixture) != KNOWN_SHA256:
        raise ValueError("Pinned known-good fixture digest mismatch")
    url = f"https://crates.io/api/v1/crates/{KNOWN_NAME}/{KNOWN_VERSION}"
    metadata = json.loads(http_get(url))["version"]
    raw = http_get(url + "/download")
    if metadata["checksum"] != KNOWN_SHA256 or raw != fixture.read_bytes():
        raise ValueError("Known-good registry download differs from trusted fixture/checksum")
    summary(f"Known-good {KNOWN_NAME} {KNOWN_VERSION}: sha256:{KNOWN_SHA256}; candidate remains unsigned")


def verify_attestations(directory, value):
    root = Path(directory)
    validate_bundle(root, value)
    bundle = root / f"{NAME}-{value}.intoto.jsonl"
    if bundle.is_symlink() or not bundle.is_file() or not bundle.stat().st_size:
        raise ValueError("Missing frozen provenance bundle")
    for name in sorted(expected_files(value)):
        subprocess.run(["gh", "attestation", "verify", str(root / name), "--repo", REPO,
                        "--bundle", str(bundle), "--signer-workflow", WORKFLOW,
                        "--source-ref", f"refs/tags/v{value}", "--source-digest", os.environ["RELEASE_COMMIT"],
                        "--deny-self-hosted-runners", "--predicate-type", "https://slsa.dev/provenance/v1"], check=True)


def release_list():
    pages = json.loads(run("gh", "api", "--paginate", "--slurp", f"repos/{REPO}/releases?per_page=100"))
    return [item for page in pages for item in page]


def semantic_version(value):
    match = re.fullmatch(r"v?(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-([0-9A-Za-z.-]+))?(?:\+[0-9A-Za-z.-]+)?", value)
    if not match:
        return None
    return tuple(int(match[i]) for i in (1, 2, 3)), match[4] is not None


def latest_policy(value, releases):
    """Compare SemVer, never upload time; unpublished drafts are not Latest."""
    target = semantic_version(value)
    if target is None:
        raise ValueError("Invalid semantic release version")
    core, prerelease = target
    stable_versions = [core]
    for release in releases:
        parsed = semantic_version(release["tag_name"])
        if parsed and not parsed[1] and not release.get("draft") and not release.get("prerelease"):
            stable_versions.append(parsed[0])
    return prerelease, not prerelease and core == max(stable_versions)


def prepare(directory, value, toolchain):
    require_publication(value)
    raw = http_get(f"https://crates.io/api/v1/crates/{NAME}")
    if raw is None or json.loads(raw)["crate"].get("trustpub_only") is not True:
        raise ValueError("Owner first-upload bootstrap and Trusted Publishing only are required; CI never bootstraps")
    verify_attestations(directory, value)
    repackage(directory, value, toolchain)
    existing = registry_metadata(value)
    if existing is not None:
        registry_compare(value, Path(directory) / f"{NAME}-{value}.crate")
    output(upload=str(existing is None).lower())


def registry_wait(directory, value):
    for attempt in range(10):
        if registry_metadata(value) is not None:
            registry_compare(value, Path(directory) / f"{NAME}-{value}.crate")
            return
        if attempt < 9:
            time.sleep(3)
    raise ValueError("Published crate did not appear within the bounded registry wait")


def release_assets(directory, value):
    require_publication(value)
    root = Path(directory).resolve()
    verify_attestations(root, value)
    bundle = root / f"{NAME}-{value}.intoto.jsonl"
    tag = f"v{value}"
    if tag_commit(tag) != os.environ["RELEASE_COMMIT"]:
        raise ValueError("Remote tag moved from the gate-verified commit")
    notes, title = release_record(value)
    # The list endpoint includes drafts that a tag lookup may not expose.
    matches = [item for item in release_list() if item["tag_name"] == tag]
    if len(matches) > 1:
        raise ValueError("Multiple Releases for the tag")
    if matches:
        existing = matches[0]
        if existing["name"] != title or (existing.get("body") or "").replace("\r\n", "\n").strip() != notes.read_text().strip():
            raise ValueError("Existing Release title/body differs from committed notes")
    else:
        prerelease, _ = latest_policy(value, [])
        command = ["gh", "release", "create", tag, "--repo", REPO, "--verify-tag", "--draft",
                   # Drafts cannot be Latest; select Latest explicitly when publishing below.
                   "--latest=false", "--target", os.environ["RELEASE_COMMIT"],
                   "--title", title, "--notes-file", str(notes)]
        if prerelease:
            command.append("--prerelease")
        subprocess.run(command, check=True)
    # Use the numeric release/asset APIs: tag lookup can hide a resumed draft.
    if not matches:
        matches = [item for item in release_list() if item["tag_name"] == tag]
    if len(matches) != 1:
        raise ValueError("Created draft could not be uniquely identified")
    release = matches[0]
    if release["name"] != title or (release.get("body") or "").replace("\r\n", "\n").strip() != notes.read_text().strip():
        raise ValueError("Release metadata differs from committed notes")
    names = expected_files(value) | {"SHA256SUMS", bundle.name}
    check_release_metadata(release, value, title, notes.read_text(), draft=release["draft"])
    if not release["draft"]:
        check_release_assets(release, root, names)
        summary("Published release already matches; no metadata or assets modified")
        return
    assets = {asset["name"]: asset for asset in release["assets"]}
    if len(assets) != len(release["assets"]) or set(assets) - names:
        raise ValueError("Duplicate/unexpected existing Release assets")
    for name in sorted(names):
        if name in assets:
            raw = subprocess.check_output(["gh", "api", f"repos/{REPO}/releases/assets/{assets[name]['id']}",
                                           "-H", "Accept: application/octet-stream"])
            if raw != (root / name).read_bytes():
                raise ValueError(f"Existing Release asset differs: {name}")
        else:
            # POST without deleting/replacing an asset: name conflicts fail closed.
            subprocess.run(["gh", "api", "--method", "POST",
                            f"https://uploads.github.com/repos/{REPO}/releases/{release['id']}/assets?name={urllib.parse.quote(name, safe='')}",
                            "-H", "Content-Type: application/octet-stream", "--input", str(root / name)],
                           check=True, stdout=subprocess.DEVNULL)
    staged = gh_api(f"repos/{REPO}/releases/{release['id']}")
    check_release_metadata(staged, value, title, notes.read_text(), draft=True)
    check_release_assets(staged, root, names)
    # Refresh after asset transfer: an older late upload must not take Latest.
    prerelease, latest = latest_policy(value, release_list())
    if tag_commit(tag) != os.environ["RELEASE_COMMIT"]:
        raise ValueError("Remote tag moved before publishing the release metadata")
    subprocess.run(["gh", "api", "--method", "PATCH", f"repos/{REPO}/releases/{release['id']}",
                    "--input", "-"], input=json.dumps({"draft": False, "prerelease": prerelease,
                                                     "make_latest": str(latest).lower()}),
                   text=True, check=True, stdout=subprocess.DEVNULL)
    published = gh_api(f"repos/{REPO}/releases/{release['id']}")
    check_release_metadata(published, value, title, notes.read_text(), draft=False)
    check_release_assets(published, root, names)


def verify_publication(directory, value):
    require_publication(value)
    root = Path(directory)
    download_release(root, value)
    frozen = Path(os.environ["RUNNER_TEMP"]) / "release-artifacts"
    for name in expected_files(value) | {"SHA256SUMS", f"{NAME}-{value}.intoto.jsonl"}:
        if (root / name).read_bytes() != (frozen / name).read_bytes():
            raise ValueError("Published asset differs from this run's frozen evidence")
    verify_attestations(root, value)
    check_archive(root / archive_name(value))
    registry_compare(value, root / f"{NAME}-{value}.crate")
    for attempt in range(10):
        raw = http_get(f"https://docs.rs/crate/{NAME}/{value}/status.json")
        if raw:
            data = json.loads(raw)
            if data.get("version") == value and data.get("doc_status") is True:
                summary(f"docs.rs {value}: passed (doc_status=true)")
                return
        if attempt < 9:
            time.sleep(10)
    summary(f"docs.rs {value}: pending/unsuccessful; no successful build observed before deadline")
    raise ValueError("docs.rs success is required; pending is not passed")


def archive_name(value):
    return f"ch10r-v{value}-windows-x86_64.zip"


def binary_names():
    """Every explicit/autodiscovered binary target, not an arbitrarily chosen executable."""
    metadata = json.loads(run("cargo", "metadata", "--locked", "--no-deps", "--format-version", "1"))
    package = next(p for p in metadata["packages"] if p["name"] == NAME)
    names = sorted(t["name"] for t in package["targets"] if "bin" in t["kind"])
    if not names or any(not re.fullmatch(r"[A-Za-z0-9_-]+", n) for n in names):
        raise ValueError("No binary targets or unsafe binary target name")
    return names


def check_archive(path, binaries=None):
    names = binaries or [t["name"] for t in tomllib.loads(Path("Cargo.toml").read_text())["bin"]]
    expected = {name + ".exe" for name in names} | {"LICENSE", "README.md"}
    with zipfile.ZipFile(path) as archive:
        if set(archive.namelist()) != expected or len(archive.namelist()) != len(expected):
            raise ValueError("Archive members differ from qualified binary targets/LICENSE/README")
        for name in names:
            if not archive.read(name + ".exe").startswith(b"MZ"):
                raise ValueError("Windows executable missing MZ header")
        if archive.testzip() is not None:
            raise ValueError("Invalid archive CRC")


def smoke_archive(path, value):
    check_archive(path)
    with tempfile.TemporaryDirectory() as directory, zipfile.ZipFile(path) as archive:
        archive.extractall(directory)  # check_archive has already required exact flat members.
        for exe in sorted(Path(directory).glob("*.exe")):
            text = run(str(exe), "--version")
            if value not in text or NAME not in text:
                raise ValueError("Executable --version differs from release identity")
            if "Usage:" not in run(str(exe), "--help"):
                raise ValueError("Executable --help failed")
    summary(f"Windows archive smoke passed: {value}")


def package_binaries(directory, value):
    root = Path(directory)
    root.mkdir(parents=True, exist_ok=True)
    names = binary_names()
    with zipfile.ZipFile(root / archive_name(value), "w", zipfile.ZIP_DEFLATED) as archive:
        for name in names:
            archive.write(Path("target/x86_64-pc-windows-msvc/release") / (name + ".exe"), name + ".exe")
        for name in ["LICENSE", "README.md"]:
            archive.write(name, name)
    check_archive(root / archive_name(value), names)
    smoke_archive(root / archive_name(value), value)


def assemble(directory, value):
    root = Path(directory)
    if {p.name for p in root.iterdir()} != expected_files(value):
        raise ValueError("Qualified payload does not match exact expected files")
    check_archive(root / archive_name(value))
    (root / "SHA256SUMS").write_text("".join(f"{digest(root / name)}  {name}\n" for name in sorted(expected_files(value))))
    validate_bundle(root, value)


def require_publication(value):
    """Defense in depth for script entry points as well as independent YAML job guards."""
    if (os.environ.get("GITHUB_EVENT_NAME") != "push"
            or os.environ.get("GITHUB_REF_TYPE") != "tag"
            or os.environ.get("GITHUB_REF") != f"refs/tags/v{value}"
            or value in legacy_records()):
        raise ValueError("Publication requires a new authorized version-tag push; dry runs/historical releases cannot publish")
    if run("git", "rev-parse", "HEAD") != os.environ["GITHUB_SHA"]:
        raise ValueError("Checkout does not match immutable event SHA")
    if os.environ["RELEASE_COMMIT"] != os.environ["GITHUB_SHA"]:
        raise ValueError("Gate evidence differs from immutable event SHA")
    if tag_commit(f"v{value}") != os.environ["GITHUB_SHA"]:
        raise ValueError("Remote annotated tag differs from immutable event SHA")


def check_release_metadata(release, value, title, body, draft):
    prerelease, _ = latest_policy(value, [])
    if (release["tag_name"] != f"v{value}" or release["name"] != title
            or (release.get("body") or "").replace("\r\n", "\n") != body.replace("\r\n", "\n")
            or release["draft"] is not draft or release["prerelease"] is not prerelease):
        raise ValueError("Release tag/title/exact committed body/draft/prerelease state differs")


def check_release_assets(release, root, names):
    assets = {a["name"]: a for a in release["assets"]}
    if set(assets) != set(names) or len(assets) != len(release["assets"]):
        raise ValueError("Release asset set differs (missing/extra/duplicate)")
    for name, asset in assets.items():
        raw = subprocess.check_output(["gh", "api", f"repos/{REPO}/releases/assets/{asset['id']}",
                                       "-H", "Accept: application/octet-stream"])
        if asset["size"] != (root / name).stat().st_size or raw != (root / name).read_bytes():
            raise ValueError(f"Release asset size/digest/bytes differ: {name}")


def download_release(directory, value):
    root = Path(directory)
    root.mkdir(parents=True, exist_ok=True)
    release = gh_api(f"repos/{REPO}/releases/tags/v{value}")
    if release["draft"] or release["tag_name"] != f"v{value}":
        raise ValueError("Expected published release tag")
    names = expected_files(value) | {"SHA256SUMS", f"{NAME}-{value}.intoto.jsonl"}
    if len(release["assets"]) != len(names) or {a["name"] for a in release["assets"]} != names:
        raise ValueError("Published asset set differs")
    for a in release["assets"]:
        raw = subprocess.check_output(["gh", "api", f"repos/{REPO}/releases/assets/{a['id']}",
                                       "-H", "Accept: application/octet-stream"])
        if len(raw) != a["size"]:
            raise ValueError("Published size differs")
        (root / a["name"]).write_bytes(raw)
    _, title = release_record(value)
    check_release_metadata(release, value, title, Path(f"docs/releases/v{value}.md").read_text(), False)
    return root


def legacy_records():
    return json.loads((Path(__file__).parent / "fixtures/legacy-releases.json").read_text())


def verify_legacy(value):
    """Accept ONLY exact historical bytes/layout, without retroactive attestation/registry claims."""
    expected = legacy_records()[value]
    release = gh_api(f"repos/{REPO}/releases/tags/v{value}")
    if release["draft"] or len(release["assets"]) != 2 or {a["name"] for a in release["assets"]} != set(expected):
        raise ValueError("Historical release layout changed")
    for asset in release["assets"]:
        raw = http_get(asset["browser_download_url"])
        if hashlib.sha256(raw).hexdigest() != expected[asset["name"]] or len(raw) != asset["size"]:
            raise ValueError("Historical release bytes changed")
    summary(f"Historical v{value}: exact pinned ZIP + sidecar accepted. No signatures/crates.io/docs.rs claimed.")
    if value == "0.1.2":
        summary("Compatibility: v0.1.2 sidecar's digest is correct but its filename says v0.1.1; unchanged historical bytes accepted ONLY by explicit pins.")


def install_check(value, toolchain):
    check_rust(toolchain)
    with tempfile.TemporaryDirectory() as directory:
        subprocess.run(["cargo", f"+{toolchain}", "install", "--locked", "--version", f"={value}",
                        "--root", directory, "--jobs", "2", NAME], check=True)
        exe = Path(directory) / "bin" / ("ch10r.exe" if os.name == "nt" else "ch10r")
        if value not in run(str(exe), "--version"):
            raise ValueError("Installed registry executable version differs")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("command", choices=["gate", "validate", "assemble", "binary-package", "smoke", "repackage", "known-good", "legacy", "prepare", "registry", "github-release", "verify", "install", "download", "signatures"])
    parser.add_argument("--directory", default="release-artifacts")
    parser.add_argument("--version")
    parser.add_argument("--rust-version")
    args = parser.parse_args()
    if args.command == "gate":
        gate()
        return
    if args.command == "known-good":
        known_good()
        return
    if args.command == "legacy":
        for value in ([args.version] if args.version else legacy_records()):
            verify_legacy(value)
        return
    value = version()
    if value != args.version:
        raise ValueError("Gate version differs from checked-out manifest")
    if args.command == "validate":
        validate_bundle(args.directory, value)
    elif args.command == "assemble":
        assemble(args.directory, value)
    elif args.command == "binary-package":
        package_binaries(args.directory, value)
    elif args.command == "smoke":
        smoke_archive(Path(args.directory) / archive_name(value), value)
    elif args.command == "repackage":
        repackage(args.directory, value, args.rust_version)
    elif args.command == "prepare":
        prepare(args.directory, value, args.rust_version)
    elif args.command == "registry":
        require_publication(value)
        registry_wait(args.directory, value)
    elif args.command == "github-release":
        release_assets(args.directory, value)
    elif args.command == "verify":
        verify_publication(args.directory, value)
    elif args.command == "download":
        require_publication(value)
        download_release(args.directory, value)
    elif args.command == "signatures":
        verify_attestations(args.directory, value)
    elif args.command == "install":
        install_check(value, args.rust_version)


if __name__ == "__main__":
    try:
        main()
    except (ValueError, FileNotFoundError, subprocess.CalledProcessError, urllib.error.URLError) as error:
        print(f"Release control failed: {error}", file=sys.stderr)
        sys.exit(1)
