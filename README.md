> **Archived** — Owner decision, 2026-11-22. This reader is superseded by
> [irig106-core](https://github.com/TelemetryWorks/irig106-core) for packet reading
> and the future `irig106 inspect` command in
> [irig106-cli](https://github.com/TelemetryWorks/irig106-cli) for inspection.
> `irig106 inspect` is future work, not a command supplied by this repository.
>
> Published Windows releases **v0.1.0–v0.1.2** and Git history are retained.
> [Download the existing releases](https://github.com/TelemetryWorks/irig106-ch10-reader/releases)
> and see the preserved [Windows usage notes](docs/windows.md).
> The [feature and test inventory](docs/FEATURES-FOR-CORE.md) is input for core's
> requirements. The original project description follows for historical reference.

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

Windows is the only supported executable target for `0.1.2`. The Linux and
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
