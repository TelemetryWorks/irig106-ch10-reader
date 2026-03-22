# irig106-ch10-reader

Fast structural reads of IRIG 106 Chapter 10 files.

`irig106-ch10-reader` is a small Rust CLI for quickly inspecting the high-level structure of `.ch10` files. It is designed to provide a lightweight structural summary without the complexity of a full IRIG 106 library or deep payload decoding.  

## What it does

Given a Chapter 10 file, the tool prints a structural inventory such as:

- channel IDs
- packet/data type distribution
- packet counts
- byte totals
- basic structural information

It is intended as a simple, focused utility for quick inspection and sanity checks.

## What it does not do

This tool is not intended to be a full-featured IRIG 106 analysis suite.

It does not aim to:

- fully decode channel payload contents
- provide rich semantic interpretation
- replace future tooling such as `irig106-cli`

## Why this exists

Sometimes you just need a fast answer to basic questions:

- What is in this `.ch10` file?
- Which channels are present?
- How many packets of each type exist?
- How large is each category of data?

This tool exists to answer those questions quickly.

## Build

```bash
cargo build --release
```

The optimized binary will be located at:

```bash
./target/release/ch10r
```

## Usage

```bash
# Summary only (default) — fast scan of the entire file
ch10r recording.ch10
# Print every packet header (verbose)
ch10r recording.ch10 --packets
# Limit to first 100 packets (useful for quick peeks at large files)
ch10r recording.ch10 --packets --limit 100
```

## What it shows

- File size and bytes parsed
- **Header checksum validation**: every packet header is verified using the IRIG 106 checksum (sum of the first 11 little-endian 16-bit words, truncated to 16 bits). This is the critical “calibration value” that distinguishes real packet headers from coincidental `0xEB25` sync patterns in payload data.
- **TMATS presence check**: warns if the required Channel 0 TMATS setup record is missing
- **Per-channel breakdown**: channel ID, data type, packet count, total data volume, min/max packet sizes
- **Per-data-type summary**: how many packets of each type (1553, PCM, Video, Ethernet, etc.)
- **Error recovery**: if sync is lost, it scans forward up to 1 MiB looking for the next position with **both** a valid sync word **and** a matching header checksum, then reports the gap

## How it works

The file is memory-mapped ( `mmap` ) so the OS handles paging — this means even multigigabyte recordings scan at near-disk-speed with minimal RAM usage. The tool walks the packet chain by reading each 24-byte header, extracting the `packet_length` field, and jumping forward to the next header. No payload decoding is performed.

## Supported data types

TMATS, PCM, Time, MIL-STD-1553, Analog, Discrete, Message, ARINC 429, Video (MPEG2/H.264/H.265/JPEG2000), Image, UART, IEEE 1394, Parallel, Ethernet, TSPI/CTS, CAN Bus,
Fibre Channel, and all Computer Generated formats.

## Header checksum

The 24-byte packet header ends with a 16-bit checksum at bytes 22–23. It’s computed as:
```
sum of words[0..11] (little-endian u16) → truncate to 16 bits
```

This value is critical for two reasons:
1. **Sync validation**: The sync word 0xEB25 is just two bytes — it will appear randomly inside payload data. The checksum confirms that the surrounding 22 bytes form a coherent header, not just a coincidence.
2. **Error recovery**: When the packet chain breaks (corruption, truncation), the tool scans forward byte-by-byte looking for a position where both the sync word matches and the checksum validates. This avoids locking onto false positives.

## Limitations

- **No payload decoding** — this is intentional. It tells you what’s in the file, not what the data says.
- **No TMATS parsing** — TMATS is ASCII text; the tool tells you it’s there and how to extract it.
- **Header checksum only** — the 16-bit header checksum is always validated. Data body checksums (8/16/32-bit, indicated in packet flags) are not checked, as that would require reading every payload byte.

## Status

Early and intentionally narrow in scope.  
  
`irig106-ch10-reader` is a focused utility for structural file inspection. Deeper inspection, richer workflows, and broader ecosystem support are expected to live in future tools such as `irig106-cli`.

