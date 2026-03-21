//! Git pkt-line encoding and decoding.
//!
//! The pkt-line format prefixes each data line with a 4-character hex
//! length (including the 4 bytes themselves). `0000` is a flush packet.
//!
//! Reference: <https://git-scm.com/docs/protocol-common#_pkt_line_format>

/// Encode a data line as a pkt-line.
///
/// Returns `{4-hex-len}{data}` where len includes the 4 prefix bytes.
pub fn encode(data: &[u8]) -> Vec<u8> {
    let len = data.len() + 4;
    let mut out = Vec::with_capacity(len);
    out.extend_from_slice(format!("{len:04x}").as_bytes());
    out.extend_from_slice(data);
    out
}

/// Encode a text line (appends newline) as a pkt-line.
pub fn encode_line(text: &str) -> Vec<u8> {
    encode(format!("{text}\n").as_bytes())
}

/// Flush packet `0000`.
pub fn flush() -> &'static [u8] {
    b"0000"
}

/// Side-band-64k channel IDs.
pub const SIDEBAND_DATA: u8 = 1;
pub const SIDEBAND_PROGRESS: u8 = 2;
#[allow(dead_code)]
pub const SIDEBAND_ERROR: u8 = 3;

/// Wrap data in a side-band-64k pkt-line (channel byte + data).
pub fn encode_sideband(channel: u8, data: &[u8]) -> Vec<u8> {
    let len = data.len() + 5; // 4 hex + 1 channel byte + data
    let mut out = Vec::with_capacity(len);
    out.extend_from_slice(format!("{len:04x}").as_bytes());
    out.push(channel);
    out.extend_from_slice(data);
    out
}

/// Read pkt-lines from a byte slice, returning each line's data and
/// advancing `pos` past the consumed bytes.
///
/// Returns `None` for a flush packet, `Some(data)` for data lines.
/// Returns `Err` if the data is malformed.
pub fn read_pktline(buf: &[u8], pos: &mut usize) -> Result<Option<Vec<u8>>, String> {
    if *pos + 4 > buf.len() {
        return Err("pktline: not enough data for length prefix".to_owned());
    }

    let hex = std::str::from_utf8(&buf[*pos..*pos + 4])
        .map_err(|_| "pktline: invalid hex prefix")?;

    if hex == "0000" {
        *pos += 4;
        return Ok(None); // Flush packet.
    }

    let len = usize::from_str_radix(hex, 16)
        .map_err(|_| format!("pktline: invalid length: {hex}"))?;

    if len < 4 {
        return Err(format!("pktline: length too small: {len}"));
    }

    if *pos + len > buf.len() {
        return Err(format!(
            "pktline: not enough data: need {len}, have {}",
            buf.len() - *pos
        ));
    }

    let data = buf[*pos + 4..*pos + len].to_vec();
    *pos += len;
    Ok(Some(data))
}

/// Read all pkt-lines until a flush packet, returning the data lines.
pub fn read_until_flush(buf: &[u8], pos: &mut usize) -> Result<Vec<Vec<u8>>, String> {
    let mut lines = Vec::new();
    loop {
        match read_pktline(buf, pos)? {
            None => break,
            Some(data) => lines.push(data),
        }
    }
    Ok(lines)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_basic() {
        let pkt = encode(b"hello\n");
        assert_eq!(&pkt, b"000ahello\n");
    }

    #[test]
    fn encode_line_adds_newline() {
        let pkt = encode_line("hello");
        assert_eq!(&pkt, b"000ahello\n");
    }

    #[test]
    fn flush_is_0000() {
        assert_eq!(flush(), b"0000");
    }

    #[test]
    fn encode_sideband_basic() {
        let pkt = encode_sideband(SIDEBAND_DATA, b"hi");
        // Length: 4 hex + 1 channel + 2 data = 7 = 0007
        assert_eq!(&pkt[..4], b"0007");
        assert_eq!(pkt[4], SIDEBAND_DATA);
        assert_eq!(&pkt[5..], b"hi");
    }

    #[test]
    fn read_pktline_data() {
        let data = b"000ahello\n";
        let mut pos = 0;
        let line = read_pktline(data, &mut pos).unwrap().unwrap();
        assert_eq!(line, b"hello\n");
        assert_eq!(pos, 10);
    }

    #[test]
    fn read_pktline_flush() {
        let data = b"0000";
        let mut pos = 0;
        let result = read_pktline(data, &mut pos).unwrap();
        assert!(result.is_none());
        assert_eq!(pos, 4);
    }

    #[test]
    fn read_until_flush_collects() {
        let mut buf = Vec::new();
        buf.extend_from_slice(&encode(b"line1\n"));
        buf.extend_from_slice(&encode(b"line2\n"));
        buf.extend_from_slice(flush());

        let mut pos = 0;
        let lines = read_until_flush(&buf, &mut pos).unwrap();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], b"line1\n");
        assert_eq!(lines[1], b"line2\n");
    }

    #[test]
    fn roundtrip() {
        let original = b"test data";
        let encoded = encode(original);
        let mut pos = 0;
        let decoded = read_pktline(&encoded, &mut pos).unwrap().unwrap();
        assert_eq!(decoded, original);
    }
}
