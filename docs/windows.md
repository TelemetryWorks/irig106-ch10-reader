# Windows

`irig106-ch10-reader` currently targets Windows first. If you are building or
testing the project on Windows, this is the primary guide to follow.

## Build

```powershell
cargo build --release
```

The binary will be written to:

```text
target\release\ch10r.exe
```

## Usage

```powershell
.\target\release\ch10r.exe --help
.\target\release\ch10r.exe --version
.\target\release\ch10r.exe recording.ch10
.\target\release\ch10r.exe recording.ch10 --packets
.\target\release\ch10r.exe recording.ch10 --packets --limit 100
```

## What the tool reports

- File size and bytes parsed
- Header checksum validation
- TMATS presence checks
- Per-channel packet and byte summaries
- Per-data-type counts
- Recovery and structural warnings when the file is damaged

## TMATS extraction note

The tool does not parse TMATS contents. It only tells you whether TMATS is
present and how large the first TMATS payload is.

If you need the raw TMATS bytes on Windows, use a dedicated binary extraction
tool or a short PowerShell script that reads from byte offset `24` for the
reported TMATS payload length.
