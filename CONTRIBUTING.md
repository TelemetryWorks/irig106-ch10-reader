# Contributing

Thanks for contributing to `irig106-ch10-reader`.

## Scope

This project is focused on fast structural inspection of IRIG 106 Chapter 10
files. Keep changes aligned with that goal.

## Development

```powershell
cargo build
cargo test
```

## Windows CI

[Windows CI](.github/workflows/windows-ci.yml) runs on pushes to `main` and pull
requests using Windows Server 2022 with MSVC. It checks Rust `1.85.0` (the minimum
supported version) and current `stable`, and logs the exact Rust/Cargo versions.
Each job checks formatting and Clippy with warnings denied, builds all targets,
runs the existing tests (including doc tests), and compiles the release executable.
Cargo builds, tests and Clippy use `--locked` to preserve `Cargo.lock`.
It also verifies that `target/release/ch10r.exe` was produced.

To reproduce a job on Windows, use an installed toolchain (`1.85.0` below, or
replace it with `stable`):

```powershell
rustup component add --toolchain 1.85.0 rustfmt clippy
rustc +1.85.0 --version --verbose
cargo +1.85.0 --version --verbose
cargo +1.85.0 fmt --check
cargo +1.85.0 clippy --locked --all-targets --jobs 2 -- -D warnings
cargo +1.85.0 build --locked --all-targets --jobs 2
cargo +1.85.0 test --locked --jobs 2 -- --test-threads=2
cargo +1.85.0 build --locked --release --jobs 2
```

CI allows at most two matrix jobs, two build workers and two test threads per
job, with a 20-minute timeout per job. Windows CI is the supported-platform
check; Linux/macOS support remains unchanged. It does not publish artifacts or
releases.

## Pull requests

- Keep changes focused and easy to review
- Update documentation when behavior changes
- Update `CHANGELOG.md` for user-facing changes
- Prefer conventional commit messages

## Issues

When reporting a bug, include:

- What command you ran
- What you expected
- What happened instead
- Whether the file was known-good, truncated, or corrupted
