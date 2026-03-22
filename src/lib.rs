use memmap2::Mmap;
use std::collections::BTreeMap;
use std::env;
use std::fmt;
use std::fs::File;
use std::process;

pub const SYNC_PATTERN: u16 = 0xEB25;
pub const HEADER_SIZE: usize = 24;

#[derive(Debug)]
pub struct PacketHeader {
    pub channel_id: u16,
    pub packet_length: u32,
    pub data_length: u32,
    pub sequence_number: u8,
    pub packet_flags: u8,
    pub data_type: u8,
    pub rtc: u64,
    pub checksum_stored: u16,
    pub checksum_valid: bool,
}

pub fn compute_header_checksum(buf: &[u8]) -> u16 {
    let mut sum: u32 = 0;
    for i in 0..11 {
        let word = u16::from_le_bytes([buf[i * 2], buf[i * 2 + 1]]);
        sum = sum.wrapping_add(word as u32);
    }
    (sum & 0xFFFF) as u16
}

pub fn is_valid_header(buf: &[u8]) -> bool {
    if buf.len() < HEADER_SIZE {
        return false;
    }
    let sync = u16::from_le_bytes([buf[0], buf[1]]);
    if sync != SYNC_PATTERN {
        return false;
    }
    let stored = u16::from_le_bytes([buf[22], buf[23]]);
    let computed = compute_header_checksum(buf);
    stored == computed
}

impl PacketHeader {
    pub fn parse(buf: &[u8]) -> Option<Self> {
        if buf.len() < HEADER_SIZE {
            return None;
        }
        let sync = u16::from_le_bytes([buf[0], buf[1]]);
        if sync != SYNC_PATTERN {
            return None;
        }
        let channel_id = u16::from_le_bytes([buf[2], buf[3]]);
        let packet_length = u32::from_le_bytes([buf[4], buf[5], buf[6], buf[7]]);
        let data_length = u32::from_le_bytes([buf[8], buf[9], buf[10], buf[11]]);
        let sequence_number = buf[13];
        let packet_flags = buf[14];
        let data_type = buf[15];
        let rtc = u64::from_le_bytes([
            buf[16], buf[17], buf[18], buf[19], buf[20], buf[21], 0, 0,
        ]);
        let checksum_stored = u16::from_le_bytes([buf[22], buf[23]]);
        let checksum_computed = compute_header_checksum(buf);

        Some(PacketHeader {
            channel_id,
            packet_length,
            data_length,
            sequence_number,
            packet_flags,
            data_type,
            rtc,
            checksum_stored,
            checksum_valid: checksum_stored == checksum_computed,
        })
    }

    pub fn has_secondary_header(&self) -> bool {
        (self.packet_flags & 0x04) != 0
    }

    pub fn checksum_type(&self) -> &'static str {
        match self.packet_flags & 0x03 {
            0 => "None",
            1 => "8-bit",
            2 => "16-bit",
            3 => "32-bit",
            _ => unreachable!(),
        }
    }

    pub fn data_overflow(&self) -> bool {
        (self.packet_flags & 0x40) != 0
    }

    pub fn rtc_sync_error(&self) -> bool {
        (self.packet_flags & 0x80) != 0
    }
}

fn data_type_name(dt: u8) -> &'static str {
    match dt {
        0x00 => "Computer Generated - Format 0 (TMATS Setup)",
        0x01 => "Computer Generated - Format 1 (Events)",
        0x02 => "Computer Generated - Format 2 (Recording Index)",
        0x03 => "Computer Generated - Format 3 (Recording Events)",
        0x04 => "Computer Generated - Format 4",
        0x09 => "PCM - Format 0 (IRIG PCM)",
        0x0A => "PCM - Format 1 (IRIG PCM Packed)",
        0x11 => "Time - Format 1 (IRIG/GPS/RTC)",
        0x12 => "Time - Format 2 (Digital)",
        0x19 => "MIL-STD-1553 - Format 1",
        0x1A => "MIL-STD-1553 - Format 2 (16PP194)",
        0x21 => "Analog - Format 1",
        0x29 => "Discrete - Format 1",
        0x30 => "Message - Format 0",
        0x38 => "ARINC 429 - Format 0",
        0x40 => "Video - Format 0 (MPEG-2 / H.264)",
        0x41 => "Video - Format 1 (JPEG 2000)",
        0x42 => "Video - Format 2 (H.264)",
        0x43 => "Video - Format 3",
        0x44 => "Video - Format 4 (H.265)",
        0x48 => "Image - Format 0 (Still Image)",
        0x49 => "Image - Format 1 (Dynamic Image)",
        0x4A => "Image - Format 2",
        0x50 => "UART - Format 0",
        0x58 => "IEEE 1394 - Format 0",
        0x59 => "IEEE 1394 - Format 1",
        0x60 => "Parallel - Format 0",
        0x68 => "Ethernet - Format 0 (MAC Frames)",
        0x69 => "Ethernet - Format 1",
        0x70 => "TSPI/CTS - Format 0",
        0x71 => "TSPI/CTS - Format 1",
        0x72 => "TSPI/CTS - Format 2",
        0x78 => "CAN Bus",
        0x79 => "Fibre Channel - Format 0",
        0x7A => "Fibre Channel - Format 1",
        _ => "Unknown / Reserved",
    }
}

fn data_type_short(dt: u8) -> &'static str {
    match dt {
        0x00 => "TMATS",
        0x01 => "CompGen-1",
        0x02 => "RecIndex",
        0x03 => "RecEvent",
        0x04 => "CompGen-4",
        0x09 => "PCM-0",
        0x0A => "PCM-1",
        0x11 => "Time-1",
        0x12 => "Time-2",
        0x19 => "1553-1",
        0x1A => "1553-2",
        0x21 => "Analog-1",
        0x29 => "Discrete-1",
        0x30 => "Message-0",
        0x38 => "ARINC429",
        0x40 => "Video-0",
        0x41 => "Video-1",
        0x42 => "Video-2",
        0x43 => "Video-3",
        0x44 => "Video-4",
        0x48 => "Image-0",
        0x49 => "Image-1",
        0x4A => "Image-2",
        0x50 => "UART-0",
        0x58 => "1394-0",
        0x59 => "1394-1",
        0x60 => "Parallel",
        0x68 => "Ether-0",
        0x69 => "Ether-1",
        0x70 => "TSPI-0",
        0x71 => "TSPI-1",
        0x72 => "TSPI-2",
        0x78 => "CAN",
        0x79 => "FibreCh-0",
        0x7A => "FibreCh-1",
        _ => "Unknown",
    }
}

pub struct ChannelStats {
    pub data_type: u8,
    pub packet_count: u64,
    pub total_data_bytes: u64,
    pub total_packet_bytes: u64,
    pub min_data_len: u32,
    pub max_data_len: u32,
    pub last_rtc: u64,
    pub overflow_count: u64,
    pub sync_error_count: u64,
    pub checksum_failures: u64,
    pub sequence_gaps: u64,
    pub last_sequence: Option<u8>,
}

impl ChannelStats {
    pub fn new(hdr: &PacketHeader) -> Self {
        ChannelStats {
            data_type: hdr.data_type,
            packet_count: 1,
            total_data_bytes: hdr.data_length as u64,
            total_packet_bytes: hdr.packet_length as u64,
            min_data_len: hdr.data_length,
            max_data_len: hdr.data_length,
            last_rtc: hdr.rtc,
            overflow_count: if hdr.data_overflow() { 1 } else { 0 },
            sync_error_count: if hdr.rtc_sync_error() { 1 } else { 0 },
            checksum_failures: if !hdr.checksum_valid { 1 } else { 0 },
            sequence_gaps: 0,
            last_sequence: Some(hdr.sequence_number),
        }
    }

    pub fn update(&mut self, hdr: &PacketHeader, packet_num: u64, errors: &mut Vec<String>) {
        self.packet_count += 1;
        self.total_data_bytes += hdr.data_length as u64;
        self.total_packet_bytes += hdr.packet_length as u64;
        self.min_data_len = self.min_data_len.min(hdr.data_length);
        self.max_data_len = self.max_data_len.max(hdr.data_length);
        self.last_rtc = hdr.rtc;
        if hdr.data_overflow() {
            self.overflow_count += 1;
        }
        if hdr.rtc_sync_error() {
            self.sync_error_count += 1;
        }
        if !hdr.checksum_valid {
            self.checksum_failures += 1;
        }
        if let Some(last_seq) = self.last_sequence {
            let expected = last_seq.wrapping_add(1);
            if hdr.sequence_number != expected {
                self.sequence_gaps += 1;
                errors.push(format!(
                    "Packet {} Ch {}: sequence gap (expected {}, got {})",
                    packet_num, hdr.channel_id, expected, hdr.sequence_number
                ));
            }
        }
        self.last_sequence = Some(hdr.sequence_number);
    }
}

struct HumanBytes(u64);

impl fmt::Display for HumanBytes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let b = self.0 as f64;
        if b < 1024.0 {
            write!(f, "{} B", self.0)
        } else if b < 1024.0 * 1024.0 {
            write!(f, "{:.1} KiB", b / 1024.0)
        } else if b < 1024.0 * 1024.0 * 1024.0 {
            write!(f, "{:.1} MiB", b / (1024.0 * 1024.0))
        } else {
            write!(f, "{:.2} GiB", b / (1024.0 * 1024.0 * 1024.0))
        }
    }
}

pub fn run_cli() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: ch10r <file.ch10> [--packets] [--limit N]");
        eprintln!();
        eprintln!("Options:");
        eprintln!("  --packets   Print every packet header (verbose)");
        eprintln!("  --limit N   Only process first N packets");
        process::exit(1);
    }

    let filepath = &args[1];
    let verbose = args.iter().any(|a| a == "--packets");
    let limit: Option<u64> = args
        .iter()
        .position(|a| a == "--limit")
        .and_then(|i| args.get(i + 1))
        .and_then(|s| s.parse().ok());

    let file = File::open(filepath).unwrap_or_else(|e| {
        eprintln!("Error: Cannot open '{}': {}", filepath, e);
        process::exit(1);
    });

    let file_size = file.metadata().unwrap().len();
    if file_size == 0 {
        eprintln!("Error: File is empty (0 bytes)");
        process::exit(1);
    }
    if file_size < HEADER_SIZE as u64 {
        eprintln!(
            "Error: File too small to contain a CH.10 packet ({} bytes, minimum is {})",
            file_size, HEADER_SIZE
        );
        process::exit(1);
    }

    let mmap = unsafe { Mmap::map(&file) }.unwrap_or_else(|e| {
        eprintln!("Error: Failed to memory-map file: {}", e);
        process::exit(1);
    });

    println!("================================================================");
    println!("IRIG 106 Chapter 10 File Inspector");
    println!("================================================================");
    println!();
    println!("  File: {}", filepath);
    println!("  Size: {} ({} bytes)", HumanBytes(file_size), file_size);
    println!();

    const MAX_RECOVERY_SCAN: usize = 1024 * 1024;

    let mut offset: usize = 0;
    let mut packet_num: u64 = 0;
    let mut channels: BTreeMap<u16, ChannelStats> = BTreeMap::new();
    let mut type_counts: BTreeMap<u8, u64> = BTreeMap::new();
    let mut errors: Vec<String> = Vec::new();
    let mut total_checksum_failures: u64 = 0;
    let mut total_sequence_gaps: u64 = 0;

    let mut first_packet_channel: Option<u16> = None;
    let mut first_packet_type: Option<u8> = None;
    let mut tmats_packet_num: Option<u64> = None;

    if verbose {
        println!(
            "  {:>8}  {:>6}  {:>10}  {:>10}  {:>6}  {:>4}  {:>12}  {:>4}  {}",
            "Packet#", "Ch ID", "PktLen", "DataLen", "Type", "Seq", "RTC", "Chk", "Data Type"
        );
        println!("  {}", "-".repeat(86));
    }

    loop {
        if offset + HEADER_SIZE > mmap.len() {
            break;
        }
        if let Some(lim) = limit {
            if packet_num >= lim {
                break;
            }
        }

        match PacketHeader::parse(&mmap[offset..]) {
            Some(hdr) => {
                if hdr.packet_length == 0 {
                    errors.push(format!(
                        "Packet {} @ offset {:#X}: packet_length is ZERO (infinite loop avoided), entering recovery",
                        packet_num, offset
                    ));
                    offset += 1;
                    continue;
                }

                if hdr.packet_length < HEADER_SIZE as u32 {
                    errors.push(format!(
                        "Packet {} @ offset {:#X}: packet_length ({}) < header size ({}), entering recovery",
                        packet_num, offset, hdr.packet_length, HEADER_SIZE
                    ));
                    offset += 1;
                    continue;
                }

                let next_offset = offset + hdr.packet_length as usize;
                if next_offset > mmap.len() {
                    let shortfall = next_offset - mmap.len();
                    errors.push(format!(
                        "Packet {} @ offset {:#X}: packet_length ({}) overruns file by {} bytes - \
                         file is truncated (recording likely interrupted)",
                        packet_num, offset, hdr.packet_length, shortfall
                    ));
                    *type_counts.entry(hdr.data_type).or_insert(0) += 1;
                    channels
                        .entry(hdr.channel_id)
                        .and_modify(|s| s.update(&hdr, packet_num, &mut errors))
                        .or_insert_with(|| ChannelStats::new(&hdr));
                    packet_num += 1;
                    break;
                }

                let max_data = hdr.packet_length.saturating_sub(HEADER_SIZE as u32);
                if hdr.data_length > max_data {
                    errors.push(format!(
                        "Packet {} @ offset {:#X}: data_length ({}) > available space ({} = pkt_len {} - hdr {})",
                        packet_num, offset, hdr.data_length, max_data, hdr.packet_length, HEADER_SIZE
                    ));
                }

                if !hdr.checksum_valid {
                    total_checksum_failures += 1;
                    errors.push(format!(
                        "Packet {} @ offset {:#X}: header checksum FAILED (stored: {:#06X}, computed: {:#06X})",
                        packet_num,
                        offset,
                        hdr.checksum_stored,
                        compute_header_checksum(&mmap[offset..offset + HEADER_SIZE])
                    ));
                }

                if packet_num == 0 {
                    first_packet_channel = Some(hdr.channel_id);
                    first_packet_type = Some(hdr.data_type);
                }
                if hdr.channel_id == 0 && hdr.data_type == 0x00 && tmats_packet_num.is_none() {
                    tmats_packet_num = Some(packet_num);
                }

                if verbose {
                    let chk_mark = if hdr.checksum_valid { "OK" } else { "BAD" };
                    println!(
                        "  {:>8}  {:>6}  {:>10}  {:>10}  {:#06X}  {:>4}  {:>12}  {:>4}  {}",
                        packet_num,
                        hdr.channel_id,
                        hdr.packet_length,
                        hdr.data_length,
                        hdr.data_type,
                        hdr.sequence_number,
                        hdr.rtc,
                        chk_mark,
                        data_type_short(hdr.data_type)
                    );
                }

                *type_counts.entry(hdr.data_type).or_insert(0) += 1;
                channels
                    .entry(hdr.channel_id)
                    .and_modify(|s| s.update(&hdr, packet_num, &mut errors))
                    .or_insert_with(|| ChannelStats::new(&hdr));

                offset = next_offset;
                packet_num += 1;
            }
            None => {
                let search_start = offset + 1;
                let mut found = false;
                let search_end = mmap.len().min(offset + MAX_RECOVERY_SCAN);
                for i in search_start..search_end.saturating_sub(HEADER_SIZE) {
                    if u16::from_le_bytes([mmap[i], mmap[i + 1]]) != SYNC_PATTERN {
                        continue;
                    }
                    if !is_valid_header(&mmap[i..]) {
                        continue;
                    }
                    let candidate = PacketHeader::parse(&mmap[i..]);
                    if let Some(ref chdr) = candidate {
                        if chdr.packet_length < HEADER_SIZE as u32 {
                            continue;
                        }
                        if (i + chdr.packet_length as usize) > mmap.len() {
                            continue;
                        }
                    } else {
                        continue;
                    }

                    errors.push(format!(
                        "Sync lost at offset {:#X}, recovered at {:#X} ({} bytes skipped) \
                         [validated by header checksum + structural checks]",
                        offset,
                        i,
                        i - offset
                    ));
                    offset = i;
                    found = true;
                    break;
                }
                if !found {
                    errors.push(format!(
                        "Sync lost at offset {:#X}, could not recover (scanned {} ahead)",
                        offset,
                        HumanBytes((search_end - offset) as u64)
                    ));
                    break;
                }
            }
        }
    }

    for stats in channels.values() {
        total_sequence_gaps += stats.sequence_gaps;
    }

    if verbose {
        println!();
    }

    println!("SUMMARY");
    println!("-------");
    println!("  Total packets: {}", packet_num);
    println!("  Channels found: {}", channels.len());
    println!(
        "  Bytes parsed: {} of {}",
        HumanBytes(offset as u64),
        HumanBytes(file_size)
    );
    if total_checksum_failures == 0 {
        println!("  Header checksums: ALL PASSED");
    } else {
        println!("  Header checksums: {} FAILED", total_checksum_failures);
    }
    if total_sequence_gaps == 0 {
        println!("  Sequence numbers: NO GAPS");
    } else {
        println!("  Sequence numbers: {} GAPS detected", total_sequence_gaps);
    }
    println!();

    println!("  CHANNELS:");
    println!(
        "  {:>6}  {:>10}  {:>10}  {:>12}  {:>8}  {:>8}  {}",
        "Ch ID", "Packets", "Type", "Total Data", "Min Pkt", "Max Pkt", "Description"
    );
    println!("  {}", "-".repeat(80));

    for (&ch_id, stats) in &channels {
        println!(
            "  {:>6}  {:>10}  {:#06X}      {:>10}  {:>8}  {:>8}  {}",
            ch_id,
            stats.packet_count,
            stats.data_type,
            HumanBytes(stats.total_data_bytes),
            stats.min_data_len,
            stats.max_data_len,
            data_type_name(stats.data_type)
        );
        if stats.overflow_count > 0
            || stats.sync_error_count > 0
            || stats.checksum_failures > 0
            || stats.sequence_gaps > 0
        {
            let mut warnings = Vec::new();
            if stats.overflow_count > 0 {
                warnings.push(format!("overflows: {}", stats.overflow_count));
            }
            if stats.sync_error_count > 0 {
                warnings.push(format!("sync errors: {}", stats.sync_error_count));
            }
            if stats.checksum_failures > 0 {
                warnings.push(format!("bad checksums: {}", stats.checksum_failures));
            }
            if stats.sequence_gaps > 0 {
                warnings.push(format!("seq gaps: {}", stats.sequence_gaps));
            }
            println!("  {:>6}  WARNING  {}", "", warnings.join(", "));
        }
    }

    println!();

    println!("  DATA TYPES:");
    println!("  {:>8}  {:>10}  {}", "Type", "Packets", "Description");
    println!("  {}", "-".repeat(60));

    for (&dt, &count) in &type_counts {
        println!("  {:#06X}    {:>10}  {}", dt, count, data_type_name(dt));
    }

    println!();

    if !errors.is_empty() {
        println!("  ISSUES ({}):", errors.len());
        for (i, e) in errors.iter().enumerate() {
            println!("    {}. {}", i + 1, e);
            if i >= 19 {
                println!("    ... and {} more", errors.len() - 20);
                break;
            }
        }
        println!();
    }

    let has_tmats = channels.get(&0).is_some_and(|s| s.data_type == 0x00);

    if has_tmats {
        let stats = channels.get(&0).unwrap();
        println!("  TMATS PRESENT on Channel 0 ({}).", HumanBytes(stats.total_data_bytes));
        println!("    TMATS contains the recording configuration and EU calibration coefficients");
        println!("    (polynomial coefficients for converting raw sensor counts to engineering units).");
        println!("    First TMATS payload length: {} bytes.", stats.min_data_len);
        println!("    For Windows extraction guidance, see docs\\windows.md.");

        match tmats_packet_num {
            Some(0) => {
                println!("    TMATS is packet #0 (correct per IRIG 106).");
            }
            Some(n) => {
                println!("    WARNING: TMATS found at packet #{} instead of packet #0.", n);
                println!("      IRIG 106 requires TMATS as the first packet. This file may have");
                println!("      been modified, concatenated, or the recorder is non-compliant.");
            }
            None => {}
        }
    } else {
        println!("  TMATS NOT FOUND - Channel 0 with data type 0x00 is missing.");
        println!("    Per IRIG 106, the first packet in a compliant CH.10 file must be TMATS.");
        println!("    Without TMATS, the following are unavailable:");
        println!("      - EU calibration coefficients (raw-to-engineering unit conversion polynomials)");
        println!("      - Channel configuration metadata (source definitions, sample rates)");
        println!("      - Recording session parameters");
        println!("    The file is still structurally parseable, but payload data cannot be");
        println!("    interpreted without external calibration documentation.");
        if let (Some(ch), Some(dt)) = (first_packet_channel, first_packet_type) {
            println!(
                "    First packet was: Channel {}, Type {:#04X} ({})",
                ch,
                dt,
                data_type_name(dt)
            );
        }
    }
    println!();
}
