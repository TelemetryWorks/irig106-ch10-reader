use irig106_ch10_reader::{
    compute_header_checksum, is_valid_header, ChannelStats, PacketHeader, HEADER_SIZE,
    SYNC_PATTERN,
};

fn make_header(
    channel_id: u16,
    packet_length: u32,
    data_length: u32,
    sequence_number: u8,
    packet_flags: u8,
    data_type: u8,
    rtc_bytes: [u8; 6],
) -> [u8; HEADER_SIZE] {
    let mut buf = [0u8; HEADER_SIZE];
    buf[0..2].copy_from_slice(&SYNC_PATTERN.to_le_bytes());
    buf[2..4].copy_from_slice(&channel_id.to_le_bytes());
    buf[4..8].copy_from_slice(&packet_length.to_le_bytes());
    buf[8..12].copy_from_slice(&data_length.to_le_bytes());
    buf[12] = 0x01;
    buf[13] = sequence_number;
    buf[14] = packet_flags;
    buf[15] = data_type;
    buf[16..22].copy_from_slice(&rtc_bytes);
    let checksum = compute_header_checksum(&buf);
    buf[22..24].copy_from_slice(&checksum.to_le_bytes());
    buf
}

#[test]
fn computes_and_validates_header_checksum() {
    let header = make_header(7, 64, 40, 3, 0x00, 0x11, [1, 2, 3, 4, 5, 6]);

    let stored = u16::from_le_bytes([header[22], header[23]]);
    assert_eq!(compute_header_checksum(&header), stored);
    assert!(is_valid_header(&header));
}

#[test]
fn rejects_header_with_bad_checksum_even_when_sync_matches() {
    let mut header = make_header(9, 48, 24, 4, 0x00, 0x19, [0, 1, 2, 3, 4, 5]);
    header[22] ^= 0xFF;

    assert!(!is_valid_header(&header));

    let parsed = PacketHeader::parse(&header).expect("sync word should still parse");
    assert!(!parsed.checksum_valid);
    assert_eq!(
        parsed.checksum_stored,
        u16::from_le_bytes([header[22], header[23]])
    );
}

#[test]
fn parses_packet_header_fields_and_flags() {
    let header = make_header(
        42,
        128,
        96,
        9,
        0xC5,
        0x40,
        [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF],
    );
    let parsed = PacketHeader::parse(&header).expect("valid header");

    assert_eq!(parsed.channel_id, 42);
    assert_eq!(parsed.packet_length, 128);
    assert_eq!(parsed.data_length, 96);
    assert_eq!(parsed.sequence_number, 9);
    assert_eq!(parsed.data_type, 0x40);
    assert!(parsed.has_secondary_header());
    assert_eq!(parsed.checksum_type(), "8-bit");
    assert!(parsed.data_overflow());
    assert!(parsed.rtc_sync_error());
    assert_eq!(parsed.rtc, 0x00FFEEDDCCBBAA);
    assert!(parsed.checksum_valid);
}

#[test]
fn parse_returns_none_for_short_or_non_sync_buffers() {
    let short = [0u8; HEADER_SIZE - 1];
    assert!(PacketHeader::parse(&short).is_none());

    let mut wrong_sync = make_header(1, 32, 8, 0, 0, 0x00, [0; 6]);
    wrong_sync[0..2].copy_from_slice(&0x0000u16.to_le_bytes());
    assert!(PacketHeader::parse(&wrong_sync).is_none());
}

#[test]
fn channel_stats_tracks_gaps_checksums_and_flags() {
    let first = PacketHeader::parse(&make_header(5, 64, 40, 7, 0x40, 0x11, [1, 0, 0, 0, 0, 0]))
        .expect("valid first header");

    let mut second_buf = make_header(5, 80, 56, 9, 0x80, 0x11, [2, 0, 0, 0, 0, 0]);
    second_buf[22] ^= 0xAA;
    let second = PacketHeader::parse(&second_buf).expect("sync should parse");

    let mut stats = ChannelStats::new(&first);
    let mut errors = Vec::new();
    stats.update(&second, 12, &mut errors);

    assert_eq!(stats.packet_count, 2);
    assert_eq!(stats.total_data_bytes, 96);
    assert_eq!(stats.total_packet_bytes, 144);
    assert_eq!(stats.min_data_len, 40);
    assert_eq!(stats.max_data_len, 56);
    assert_eq!(stats.overflow_count, 1);
    assert_eq!(stats.sync_error_count, 1);
    assert_eq!(stats.checksum_failures, 1);
    assert_eq!(stats.sequence_gaps, 1);
    assert_eq!(stats.last_sequence, Some(9));
    assert_eq!(stats.last_rtc, 2);
    assert_eq!(errors.len(), 1);
    assert!(errors[0].contains("expected 8 got 9"));
    assert!(errors[0].contains("Packet 12 Ch 5"));
}
