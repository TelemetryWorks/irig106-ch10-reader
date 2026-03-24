use super::data_type_parts;

#[test]
fn data_type_labels_match_common_chapter10_definitions() {
    assert_eq!(
        data_type_parts(0x01),
        ("Computer Generated", "Format 1", "TMATS Setup")
    );
    assert_eq!(
        data_type_parts(0x02),
        ("Computer Generated", "Format 2", "Recording Events")
    );
    assert_eq!(
        data_type_parts(0x03),
        ("Computer Generated", "Format 3", "Recording Index")
    );
    assert_eq!(data_type_parts(0x08), ("PCM", "Format 0", "Reserved"));
    assert_eq!(data_type_parts(0x09), ("PCM", "Format 1", "IRIG PCM"));
    assert_eq!(data_type_parts(0x0A), ("PCM", "Format 2", "Reserved"));
    assert_eq!(data_type_parts(0x11), ("Time", "Format 1", "IRIG/GPS/RTC"));
    assert_eq!(
        data_type_parts(0x41),
        ("Video", "Format 1", "ISO 13818-1 MPEG-2")
    );
    assert_eq!(
        data_type_parts(0x42),
        ("Video", "Format 2", "ISO 14496-10 AVC/H.264")
    );
    assert_eq!(data_type_parts(0x48), ("Image", "Format 0", "Image Data"));
    assert_eq!(
        data_type_parts(0x49),
        ("Image", "Format 1", "Still Imagery")
    );
    assert_eq!(
        data_type_parts(0x58),
        ("IEEE 1394", "Format 0", "IEEE 1394 Transaction")
    );
    assert_eq!(
        data_type_parts(0x59),
        ("IEEE 1394", "Format 1", "IEEE 1394 Physical Layer")
    );
    assert_eq!(
        data_type_parts(0x68),
        ("Ethernet", "Format 0", "Ethernet Data")
    );
    assert_eq!(
        data_type_parts(0x69),
        ("Ethernet", "Format 1", "Ethernet UDP Payload")
    );
    assert_eq!(
        data_type_parts(0x70),
        ("TSPI/CTS", "Format 0", "GPS NMEA-RTCM")
    );
    assert_eq!(
        data_type_parts(0x78),
        ("Controller Area Network", "", "CAN Bus")
    );
    assert_eq!(
        data_type_parts(0x79),
        ("Fibre Channel", "Format 0", "Fibre Channel Data")
    );
}
