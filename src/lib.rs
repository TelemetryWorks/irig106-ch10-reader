use memmap2::Mmap;
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fmt;
use std::fs::File;
use std::path::Path;
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
        let rtc = u64::from_le_bytes([buf[16], buf[17], buf[18], buf[19], buf[20], buf[21], 0, 0]);
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

#[cfg(test)]
mod lib_tests;

fn data_type_parts(dt: u8) -> (&'static str, &'static str, &'static str) {
    match dt {
        0x00 => ("Computer Generated", "Format 0", "User Defined"),
        0x01 => ("Computer Generated", "Format 1", "TMATS Setup"),
        0x02 => ("Computer Generated", "Format 2", "Recording Events"),
        0x03 => ("Computer Generated", "Format 3", "Recording Index"),
        0x04 => ("Computer Generated", "Format 4", "Reserved"),
        0x05 => ("Computer Generated", "Format 5", "Reserved"),
        0x06 => ("Computer Generated", "Format 6", "Reserved"),
        0x07 => ("Computer Generated", "Format 7", "Reserved"),
        0x08 => ("PCM", "Format 0", "Reserved"),
        0x09 => ("PCM", "Format 1", "IRIG PCM"),
        0x0A => ("PCM", "Format 2", "Reserved"),
        0x0B => ("PCM", "Format 3", "Reserved"),
        0x0C => ("PCM", "Format 4", "Reserved"),
        0x0D => ("PCM", "Format 5", "Reserved"),
        0x0E => ("PCM", "Format 6", "Reserved"),
        0x0F => ("PCM", "Format 7", "Reserved"),
        0x11 => ("Time", "Format 1", "IRIG/GPS/RTC"),
        0x12 => ("Time", "Format 2", "Digital"),
        0x19 => ("MIL-STD-1553", "Format 1", "MIL-STD-1553 Data"),
        0x1A => ("MIL-STD-1553", "Format 2", "16PP194"),
        0x21 => ("Analog", "Format 1", "Analog Data"),
        0x29 => ("Discrete", "Format 1", "Discrete Data"),
        0x30 => ("Message", "Format 0", "Generic Message Data"),
        0x38 => ("ARINC 429", "Format 0", "ARINC 429 Data"),
        0x40 => ("Video", "Format 0", "MPEG-2/H.264 Video"),
        0x41 => ("Video", "Format 1", "ISO 13818-1 MPEG-2"),
        0x42 => ("Video", "Format 2", "ISO 14496-10 AVC/H.264"),
        0x43 => ("Video", "Format 3", "Reserved"),
        0x44 => ("Video", "Format 4", "Reserved"),
        0x45 => ("Video", "Format 5", "Reserved"),
        0x46 => ("Video", "Format 6", "Reserved"),
        0x47 => ("Video", "Format 7", "Reserved"),
        0x48 => ("Image", "Format 0", "Image Data"),
        0x49 => ("Image", "Format 1", "Still Imagery"),
        0x4A => ("Image", "Format 2", "Dynamic Imagery"),
        0x4B => ("Image", "Format 3", "Reserved"),
        0x4C => ("Image", "Format 4", "Reserved"),
        0x4D => ("Image", "Format 5", "Reserved"),
        0x4E => ("Image", "Format 6", "Reserved"),
        0x4F => ("Image", "Format 7", "Reserved"),
        0x50 => ("UART", "Format 0", "UART Data"),
        0x58 => ("IEEE 1394", "Format 0", "IEEE 1394 Transaction"),
        0x59 => ("IEEE 1394", "Format 1", "IEEE 1394 Physical Layer"),
        0x60 => ("Parallel", "Format 0", "Parallel Data"),
        0x68 => ("Ethernet", "Format 0", "Ethernet Data"),
        0x69 => ("Ethernet", "Format 1", "Ethernet UDP Payload"),
        0x70 => ("TSPI/CTS", "Format 0", "GPS NMEA-RTCM"),
        0x71 => ("TSPI/CTS", "Format 1", "EAG ACMI"),
        0x72 => ("TSPI/CTS", "Format 2", "ACTTS"),
        0x78 => ("Controller Area Network", "", "CAN Bus"),
        0x79 => ("Fibre Channel", "Format 0", "Fibre Channel Data"),
        0x7A => ("Fibre Channel", "Format 1", "Reserved"),
        _ => ("Unknown / Reserved", "", ""),
    }
}

fn data_type_short(dt: u8) -> &'static str {
    match dt {
        0x00 => "CompGen-0",
        0x01 => "TMATS",
        0x02 => "RecEvent",
        0x03 => "RecIndex",
        0x04 => "CompGen-4",
        0x05 => "CompGen-5",
        0x06 => "CompGen-6",
        0x07 => "CompGen-7",
        0x08 => "PCM-0",
        0x09 => "PCM-1",
        0x0A => "PCM-2",
        0x0B => "PCM-3",
        0x0C => "PCM-4",
        0x0D => "PCM-5",
        0x0E => "PCM-6",
        0x0F => "PCM-7",
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

    pub fn update(&mut self, hdr: &PacketHeader, packet_num: u64, sequence_gaps: &mut Vec<String>) {
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
                sequence_gaps.push(format!(
                    "Packet {} Ch {} expected {} got {}",
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

struct CommaInt(u64);

impl fmt::Display for CommaInt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let digits = self.0.to_string();
        for (i, ch) in digits.chars().enumerate() {
            if i > 0 && (digits.len() - i) % 3 == 0 {
                write!(f, ",")?;
            }
            write!(f, "{ch}")?;
        }
        Ok(())
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
    let file_name = Path::new(filepath)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(filepath);
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

    const MAX_RECOVERY_SCAN: usize = 1024 * 1024;

    let mut offset: usize = 0;
    let mut packet_num: u64 = 0;
    let mut channels: BTreeMap<(u16, u8), ChannelStats> = BTreeMap::new();
    let mut channel_ids: BTreeSet<u16> = BTreeSet::new();
    let mut issues: Vec<String> = Vec::new();
    let mut checksum_failures: Vec<String> = Vec::new();
    let mut sequence_gaps: Vec<String> = Vec::new();
    let mut total_checksum_failures: u64 = 0;
    let mut total_sequence_gaps: u64 = 0;
    let mut has_tmats = false;
    let mut verbose_rows: Vec<String> = Vec::new();

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
                    issues.push(format!(
                        "Packet {} @ offset {:#X}: packet_length is ZERO (infinite loop avoided), entering recovery",
                        packet_num, offset
                    ));
                    offset += 1;
                    continue;
                }

                if hdr.packet_length < HEADER_SIZE as u32 {
                    issues.push(format!(
                        "Packet {} @ offset {:#X}: packet_length ({}) < header size ({}), entering recovery",
                        packet_num, offset, hdr.packet_length, HEADER_SIZE
                    ));
                    offset += 1;
                    continue;
                }

                let next_offset = offset + hdr.packet_length as usize;
                if next_offset > mmap.len() {
                    let shortfall = next_offset - mmap.len();
                    issues.push(format!(
                        "Packet {} @ offset {:#X}: packet_length ({}) overruns file by {} bytes - file is truncated (recording likely interrupted)",
                        packet_num, offset, hdr.packet_length, shortfall
                    ));
                    channel_ids.insert(hdr.channel_id);
                    channels
                        .entry((hdr.channel_id, hdr.data_type))
                        .and_modify(|s| s.update(&hdr, packet_num, &mut sequence_gaps))
                        .or_insert_with(|| ChannelStats::new(&hdr));
                    packet_num += 1;
                    break;
                }

                let max_data = hdr.packet_length.saturating_sub(HEADER_SIZE as u32);
                if hdr.data_length > max_data {
                    issues.push(format!(
                        "Packet {} @ offset {:#X}: data_length ({}) > available space ({} = pkt_len {} - hdr {})",
                        packet_num, offset, hdr.data_length, max_data, hdr.packet_length, HEADER_SIZE
                    ));
                }

                if !hdr.checksum_valid {
                    total_checksum_failures += 1;
                    checksum_failures.push(format!(
                        "Packet {} @ offset {:#X} stored {:#06X} computed {:#06X}",
                        packet_num,
                        offset,
                        hdr.checksum_stored,
                        compute_header_checksum(&mmap[offset..offset + HEADER_SIZE])
                    ));
                }

                if hdr.channel_id == 0 && hdr.data_type == 0x01 {
                    has_tmats = true;
                }

                if verbose {
                    let chk_mark = if hdr.checksum_valid { "OK" } else { "BAD" };
                    verbose_rows.push(format!(
                        "{:>8}  {:>6}  {:>10}  {:>10}  {:#06X}  {:>4}  {:>12}  {:>4}  {}",
                        packet_num,
                        hdr.channel_id,
                        hdr.packet_length,
                        hdr.data_length,
                        hdr.data_type,
                        hdr.sequence_number,
                        hdr.rtc,
                        chk_mark,
                        data_type_short(hdr.data_type)
                    ));
                }

                channel_ids.insert(hdr.channel_id);
                channels
                    .entry((hdr.channel_id, hdr.data_type))
                    .and_modify(|s| s.update(&hdr, packet_num, &mut sequence_gaps))
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

                    issues.push(format!(
                        "Sync lost at offset {:#X}, recovered at {:#X} ({} bytes skipped) [validated by header checksum + structural checks]",
                        offset,
                        i,
                        i - offset
                    ));
                    offset = i;
                    found = true;
                    break;
                }
                if !found {
                    issues.push(format!(
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

    println!("================================================================");
    println!("IRIG 106 Chapter 10 Structural Reader");
    println!("================================================================");
    println!();
    println!("File Name                : {}", file_name);
    println!("File Size                : {} bytes", CommaInt(file_size));
    println!("Packets                  : {}", CommaInt(packet_num));
    println!("Channels                 : {}", channel_ids.len());
    println!("Recording Date / Time    : Not available");
    println!("Data Start Time          : Not available");
    println!("Data Stop Time           : Not available");
    if total_checksum_failures == 0 {
        println!("Header Checksums         : All passed");
    } else {
        println!(
            "Header Checksums         : {} failed",
            total_checksum_failures
        );
        for detail in &checksum_failures {
            println!("                           {}", detail);
        }
    }
    if total_sequence_gaps == 0 {
        println!("Sequence Numbers         : No gaps");
    } else {
        println!(
            "Sequence Numbers         : {} gaps detected",
            total_sequence_gaps
        );
        for detail in &sequence_gaps {
            println!("                           {}", detail);
        }
    }
    println!(
        "TMATS                    : {}",
        if has_tmats { "Present" } else { "Not found" }
    );
    println!();

    println!(
        "{:<7}  {:<22}  {:<12}  {:<22}  {:<8}  {:>12}  {:>10}  {:>8}  {:>8}",
        "Channel",
        "Data Type",
        "Data Format",
        "Data Detail",
        "Type Num",
        "Packet Count",
        "Data Bytes",
        "Min Data",
        "Max Data"
    );
    println!("{}", "-".repeat(129));
    for (&(ch_id, _data_type), stats) in &channels {
        let (type_name, format_name, detail_name) = data_type_parts(stats.data_type);
        let type_num = format!("0x{:02X}", stats.data_type);
        let packet_count = CommaInt(stats.packet_count).to_string();
        let data_bytes = HumanBytes(stats.total_data_bytes).to_string();
        println!(
            "{:>7}  {:<22}  {:<12}  {:<22}  {:>8}  {:>12}  {:>10}  {:>8}  {:>8}",
            ch_id,
            type_name,
            format_name,
            detail_name,
            type_num,
            packet_count,
            data_bytes,
            stats.min_data_len,
            stats.max_data_len
        );
    }
    println!();

    if !issues.is_empty() {
        println!("Issues");
        println!("------");
        for (i, issue) in issues.iter().enumerate() {
            println!("{}. {}", i + 1, issue);
            if i >= 19 {
                println!("... and {} more", issues.len() - 20);
                break;
            }
        }
        println!();
    }

    if verbose {
        println!("Packet Listing");
        println!("--------------");
        println!(
            "{:>8}  {:>6}  {:>10}  {:>10}  {:>6}  {:>4}  {:>12}  {:>4}  {}",
            "Packet#", "Ch ID", "PktLen", "DataLen", "Type", "Seq", "RTC", "Chk", "Data Type"
        );
        println!("{}", "-".repeat(86));
        for row in &verbose_rows {
            println!("{row}");
        }
        println!();
    }
}
