//! Git pack file encoding and decoding.
//!
//! Supports only full objects (types 1-4: commit, tree, blob, tag).
//! Delta objects (ofs-delta, ref-delta) are NOT supported — the server
//! does not advertise `ofs-delta` capability, so clients send full objects.
//!
//! Reference: <https://git-scm.com/docs/pack-format>

use std::io::{Read, Write};

use corp_store::store::GitObjectType;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use sha1::Digest;

/// Maximum size of a single blob object (10 MB).
/// Corp data is structured JSON/text — anything larger is likely a mistake.
pub const MAX_BLOB_SIZE: u64 = 10 * 1024 * 1024;

/// Maximum total pack upload size (2 GB).
/// This caps the raw data a single push can transmit.
pub const MAX_PACK_SIZE: usize = 2 * 1024 * 1024 * 1024;

/// A parsed git object from a pack.
#[derive(Debug, Clone)]
pub struct PackObject {
    pub obj_type: GitObjectType,
    pub content: Vec<u8>,
    pub sha1_hex: String,
}

// ── Pack parsing (receive-pack) ──────────────────────────────────────

/// Parse a pack stream into individual git objects.
///
/// Expects the pack to start with `PACK`, version 2, object count,
/// followed by zlib-compressed object entries, and a trailing 20-byte
/// SHA-1 checksum.
///
/// Only full objects (types 1-4) are supported. Delta types (5-7) will
/// return an error — the server should NOT advertise `ofs-delta` to
/// prevent clients from sending deltas.
pub fn parse_pack(data: &[u8]) -> Result<Vec<PackObject>, String> {
    if data.len() > MAX_PACK_SIZE {
        return Err(format!(
            "pack size {} bytes exceeds maximum allowed {} bytes ({})",
            data.len(),
            MAX_PACK_SIZE,
            "2 GB"
        ));
    }
    if data.len() < 12 {
        return Err("pack too short for header".to_owned());
    }

    // Verify magic.
    if &data[0..4] != b"PACK" {
        return Err("invalid pack magic".to_owned());
    }

    // Version (network byte order).
    let version = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
    if version != 2 && version != 3 {
        return Err(format!("unsupported pack version: {version}"));
    }

    // Object count.
    let num_objects = u32::from_be_bytes([data[8], data[9], data[10], data[11]]);

    let mut pos = 12;
    let mut objects = Vec::with_capacity(num_objects as usize);

    for _ in 0..num_objects {
        if pos >= data.len() {
            return Err("pack truncated: not enough data for next object".to_owned());
        }

        // Read variable-length object header.
        // First byte: bits 6-4 = type, bits 3-0 = size (low 4 bits).
        // Subsequent bytes (if MSB set): 7 more size bits each.
        let first = data[pos];
        pos += 1;

        let type_num = (first >> 4) & 0x07;
        let mut size: u64 = (first & 0x0f) as u64;
        let mut shift = 4;

        let mut byte = first;
        while byte & 0x80 != 0 {
            if pos >= data.len() {
                return Err("pack truncated in object header".to_owned());
            }
            byte = data[pos];
            pos += 1;
            size |= ((byte & 0x7f) as u64) << shift;
            shift += 7;
        }

        let obj_type = match type_num {
            1 => GitObjectType::Commit,
            2 => GitObjectType::Tree,
            3 => GitObjectType::Blob,
            4 => return Err("tag objects not supported".to_owned()),
            5 => return Err("ofs-delta not supported (don't advertise ofs-delta)".to_owned()),
            6 => return Err("ofs-delta not supported (don't advertise ofs-delta)".to_owned()),
            7 => return Err("ref-delta not supported (use no-thin)".to_owned()),
            _ => return Err(format!("unknown object type: {type_num}")),
        };

        // Decompress the object data.
        let mut decoder = ZlibDecoder::new(&data[pos..]);
        let mut content = Vec::with_capacity(size as usize);
        decoder
            .read_to_end(&mut content)
            .map_err(|e| format!("zlib decompression failed: {e}"))?;

        if content.len() != size as usize {
            return Err(format!(
                "decompressed size mismatch: expected {size}, got {}",
                content.len()
            ));
        }

        // Enforce blob size limit.
        if obj_type == GitObjectType::Blob && size > MAX_BLOB_SIZE {
            return Err(format!(
                "blob object is {} bytes, which exceeds the {} byte limit (10 MB). \
                 Corp repos are for structured data — large files should be stored elsewhere.",
                size, MAX_BLOB_SIZE
            ));
        }

        // Advance past the compressed data.
        pos += decoder.total_in() as usize;

        // Compute SHA-1 of the full git object (header + content).
        let header = format!("{} {}\0", obj_type.as_str(), content.len());
        let mut hasher = sha1::Sha1::new();
        hasher.update(header.as_bytes());
        hasher.update(&content);
        let sha1_hex = hex::encode(hasher.finalize());

        objects.push(PackObject {
            obj_type,
            content,
            sha1_hex,
        });
    }

    Ok(objects)
}

// ── Pack generation (upload-pack) ────────────────────────────────────

/// Build a pack file from full objects (no deltas).
///
/// The pack contains only undeltified objects (types 1-4), which any
/// git client can decode. Delta compression can be added later for
/// bandwidth optimization.
pub fn build_pack(objects: &[(String, GitObjectType, Vec<u8>)]) -> Result<Vec<u8>, String> {
    let mut buf = Vec::new();

    // PACK header.
    buf.extend_from_slice(b"PACK");
    buf.extend_from_slice(&2u32.to_be_bytes()); // version 2
    buf.extend_from_slice(&(objects.len() as u32).to_be_bytes());

    for (_sha1, obj_type, content) in objects {
        // Encode object header (variable-length).
        encode_pack_object_header(&mut buf, obj_type.pack_type(), content.len() as u64);

        // Compress the content.
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder
            .write_all(content)
            .map_err(|e| format!("zlib compression failed: {e}"))?;
        let compressed = encoder
            .finish()
            .map_err(|e| format!("zlib finish failed: {e}"))?;
        buf.extend_from_slice(&compressed);
    }

    // Trailing SHA-1 checksum of the entire pack (excluding the checksum itself).
    let mut hasher = sha1::Sha1::new();
    hasher.update(&buf);
    let checksum = hasher.finalize();
    buf.extend_from_slice(&checksum);

    Ok(buf)
}

/// Encode a pack object header (type + uncompressed size) as a variable-length integer.
fn encode_pack_object_header(buf: &mut Vec<u8>, type_num: u8, size: u64) {
    // First byte: bits 6-4 = type, bits 3-0 = low 4 bits of size.
    let mut first = (type_num << 4) | (size as u8 & 0x0f);
    let mut remaining = size >> 4;

    if remaining > 0 {
        first |= 0x80; // More bytes follow.
    }
    buf.push(first);

    while remaining > 0 {
        let mut byte = (remaining & 0x7f) as u8;
        remaining >>= 7;
        if remaining > 0 {
            byte |= 0x80;
        }
        buf.push(byte);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_single_blob() {
        let content = b"hello world";
        let sha1_hex = {
            let header = format!("blob {}\0", content.len());
            let mut hasher = sha1::Sha1::new();
            hasher.update(header.as_bytes());
            hasher.update(content);
            hex::encode(hasher.finalize())
        };

        let objects = vec![(sha1_hex.clone(), GitObjectType::Blob, content.to_vec())];
        let pack = build_pack(&objects).unwrap();

        let parsed = parse_pack(&pack).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].obj_type, GitObjectType::Blob);
        assert_eq!(parsed[0].content, content);
        assert_eq!(parsed[0].sha1_hex, sha1_hex);
    }

    #[test]
    fn roundtrip_multiple_objects() {
        let blob1 = b"content one";
        let blob2 = b"content two";
        let objects = vec![
            (String::new(), GitObjectType::Blob, blob1.to_vec()),
            (String::new(), GitObjectType::Blob, blob2.to_vec()),
            (String::new(), GitObjectType::Commit, b"tree 0000000000000000000000000000000000000000\nauthor A <a@a> 0 +0000\ncommitter A <a@a> 0 +0000\n\ninit\n".to_vec()),
        ];
        let pack = build_pack(&objects).unwrap();

        let parsed = parse_pack(&pack).unwrap();
        assert_eq!(parsed.len(), 3);
        assert_eq!(parsed[0].obj_type, GitObjectType::Blob);
        assert_eq!(parsed[0].content, blob1);
        assert_eq!(parsed[1].obj_type, GitObjectType::Blob);
        assert_eq!(parsed[1].content, blob2);
        assert_eq!(parsed[2].obj_type, GitObjectType::Commit);
    }

    #[test]
    fn header_encoding_small() {
        let mut buf = Vec::new();
        // Blob type (3), size 10 — fits in 4 bits, no continuation.
        encode_pack_object_header(&mut buf, 3, 10);
        assert_eq!(buf.len(), 1);
        assert_eq!(buf[0] & 0x80, 0); // No continuation bit.
        assert_eq!((buf[0] >> 4) & 0x07, 3); // Type = 3.
        assert_eq!(buf[0] & 0x0f, 10); // Size = 10.
    }

    #[test]
    fn header_encoding_large() {
        let mut buf = Vec::new();
        // Blob type (3), size 1000 — needs continuation bytes.
        encode_pack_object_header(&mut buf, 3, 1000);
        assert!(buf.len() > 1);
        assert_eq!(buf[0] & 0x80, 0x80); // Continuation bit set.
    }

    #[test]
    fn invalid_pack_magic() {
        let err = parse_pack(b"NOPE\x00\x00\x00\x02\x00\x00\x00\x00").unwrap_err();
        assert!(err.contains("invalid pack magic"));
    }
}
