# irig106-ch10-reader

Fast structural reads of IRIG 106 Chapter 10 files.

`irig106-ch10-reader` is a small Rust CLI for quickly inspecting the high-level
structure of `.ch10` files. It is intentionally narrow in scope: fast
structural summaries, not deep payload decoding.

## What it does

- Reports channel IDs, packet counts, and byte totals
- Summarizes data types found in the recording
- Validates packet header checksums
- Detects sequence gaps and several structural issues
- Checks for the required TMATS setup record
- Attempts sync recovery when the packet chain breaks

## Platform docs

- [Windows](docs/windows.md)
- [Linux](docs/linux.md)
- [macOS](docs/macos.md)

Windows is the only supported executable target for `0.1.0`. The Linux and
macOS guides are placeholders for future support planning.

## Build

```powershell
cargo build --release
```

The Windows release binary is:

```text
target\release\ch10r.exe
```

## Status

Early and intentionally narrow in scope.

Deeper inspection, richer workflows, and broader ecosystem support are
expected to live in future tools such as `irig106-cli`.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).
