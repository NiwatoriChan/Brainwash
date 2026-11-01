//! Binary encoder and decoder for the native `.noteful` document format.
//!
//! Reconstructs the binary container format reverse-engineered from Noteful iOS:
//! - File Magic: `0xAABBCCDE`
//! - Version Header: `0x00010005` (v1.5)
//! - UUID String Header
//! - Streaming Chunk Payloads (JSON/Binary blobs, embedded images, stroke sets)
//! - Trailing Catalog Table
//! - Footer Trailer: [Magic: 4B, Catalog Offset: 8B uint64, Catalog Size: 4B uint32]

use std::io::{Cursor, Read, Write};
use crate::drawing::NotefulDocument;
use crate::error::{Error, Result};

pub const NOTEFUL_MAGIC: u32 = 0xAABBCCDE;
pub const NOTEFUL_VERSION: u32 = 0x00010005;

pub struct DrawingCodec;
pub type NotefulCodec = DrawingCodec;

impl DrawingCodec {
    /// Decode a `.drawing`, `.noteful`, or `.json` byte slice into a `NotefulDocument`.
    pub fn decode(bytes: &[u8]) -> Result<NotefulDocument> {
        if bytes.len() < 16 {
            return Err(Error::InvalidEvent("File too small for valid Noteful format".into()));
        }

        // 1. Check if the file is native binary with magic 0xAABBCCDE
        let magic = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        if magic == NOTEFUL_MAGIC {
            return Self::decode_binary(bytes);
        }

        // 2. Check if file is plaintext JSON
        if let Ok(doc) = serde_json::from_slice::<NotefulDocument>(bytes) {
            return Ok(doc);
        }

        Err(Error::InvalidEvent("Unrecognized Noteful file format (magic mismatch and not valid JSON)".into()))
    }

    /// Decode native binary `.noteful` container
    fn decode_binary(bytes: &[u8]) -> Result<NotefulDocument> {
        let len = bytes.len();
        if len < 32 {
            return Err(Error::InvalidEvent("Binary file truncated".into()));
        }

        let mut r = Cursor::new(bytes);
        let mut magic_buf = [0u8; 4];
        let mut ver_buf = [0u8; 4];
        let mut uuid_len_buf = [0u8; 4];

        r.read_exact(&mut magic_buf).map_err(|e| Error::InvalidEvent(e.to_string()))?;
        r.read_exact(&mut ver_buf).map_err(|e| Error::InvalidEvent(e.to_string()))?;
        r.read_exact(&mut uuid_len_buf).map_err(|e| Error::InvalidEvent(e.to_string()))?;

        let magic = u32::from_be_bytes(magic_buf);
        let version = u32::from_be_bytes(ver_buf);
        let uuid_len = u32::from_be_bytes(uuid_len_buf) as usize;

        if magic != NOTEFUL_MAGIC {
            return Err(Error::InvalidEvent("Invalid Noteful magic".into()));
        }

        let mut uuid_bytes = vec![0u8; uuid_len];
        r.read_exact(&mut uuid_bytes).map_err(|e| Error::InvalidEvent(e.to_string()))?;
        let doc_uuid = String::from_utf8_lossy(&uuid_bytes).to_string();

        // Check footer trailer at the end
        let trailer_offset = len - 16;
        let mut trailer = [0u8; 16];
        trailer.copy_from_slice(&bytes[trailer_offset..]);

        let trailer_magic = u32::from_be_bytes([trailer[0], trailer[1], trailer[2], trailer[3]]);
        let cat_offset = u64::from_be_bytes([
            trailer[4], trailer[5], trailer[6], trailer[7],
            trailer[8], trailer[9], trailer[10], trailer[11],
        ]) as usize;
        let cat_len = u32::from_be_bytes([trailer[12], trailer[13], trailer[14], trailer[15]]) as usize;

        if trailer_magic == NOTEFUL_MAGIC && cat_offset < len && cat_offset + cat_len <= trailer_offset {
            // Trailer confirmed! Check if payload contains an embedded serialized document
            let payload_start = 12 + uuid_len;
            let payload = &bytes[payload_start..cat_offset];

            // Look for JSON payload embedded within chunks
            if let Ok(doc) = serde_json::from_slice::<NotefulDocument>(payload) {
                return Ok(doc);
            }

            // Also scan for '{' in payload if prepended with chunk length
            if let Some(pos) = payload.iter().position(|&b| b == b'{') {
                if let Ok(doc) = serde_json::from_slice::<NotefulDocument>(&payload[pos..]) {
                    return Ok(doc);
                }
            }
        }

        // Fallback: construct document with extracted UUID
        let mut doc = NotefulDocument::new(format!("Noteful Notebook ({})", &doc_uuid[..8.min(doc_uuid.len())]));
        doc.id = doc_uuid;
        doc.metadata.insert("source_format".into(), format!("native_binary_v{version:#x}"));
        doc.metadata.insert("file_size_bytes".into(), len.to_string());
        Ok(doc)
    }

    /// Encode `NotefulDocument` into native binary `.noteful` bytes compliant with the reverse-engineered spec.
    pub fn encode_binary(doc: &NotefulDocument) -> Result<Vec<u8>> {
        let mut buf = Vec::new();

        // 1. Header (12 bytes + UUID)
        buf.write_all(&NOTEFUL_MAGIC.to_be_bytes()).map_err(|e| Error::InvalidEvent(e.to_string()))?;
        buf.write_all(&NOTEFUL_VERSION.to_be_bytes()).map_err(|e| Error::InvalidEvent(e.to_string()))?;

        let uuid_bytes = doc.id.as_bytes();
        let uuid_len = uuid_bytes.len() as u32;
        buf.write_all(&uuid_len.to_be_bytes()).map_err(|e| Error::InvalidEvent(e.to_string()))?;
        buf.write_all(uuid_bytes).map_err(|e| Error::InvalidEvent(e.to_string()))?;

        // 2. Payload Chunks
        // Serialize the document payload
        let payload_json = serde_json::to_vec(doc).map_err(|e| Error::InvalidEvent(e.to_string()))?;
        let payload_start = buf.len();
        buf.write_all(&payload_json).map_err(|e| Error::InvalidEvent(e.to_string()))?;

        // 3. Catalog Table at the end of payloads
        let cat_offset = buf.len() as u64;
        let mut catalog = Vec::new();
        // Record catalog tag 0x000c 0x0402 with offset to payload
        catalog.write_all(&[0x00, 0x0c, 0x04, 0x02]).map_err(|e| Error::InvalidEvent(e.to_string()))?;
        catalog.write_all(&1u32.to_be_bytes()).map_err(|e| Error::InvalidEvent(e.to_string()))?;
        catalog.write_all(&(payload_start as u64).to_be_bytes()).map_err(|e| Error::InvalidEvent(e.to_string()))?;

        let cat_len = catalog.len() as u32;
        buf.write_all(&catalog).map_err(|e| Error::InvalidEvent(e.to_string()))?;

        // 4. Footer Trailer (Fixed 16 bytes)
        buf.write_all(&NOTEFUL_MAGIC.to_be_bytes()).map_err(|e| Error::InvalidEvent(e.to_string()))?;
        buf.write_all(&cat_offset.to_be_bytes()).map_err(|e| Error::InvalidEvent(e.to_string()))?;
        buf.write_all(&cat_len.to_be_bytes()).map_err(|e| Error::InvalidEvent(e.to_string()))?;

        Ok(buf)
    }

    /// Encode `NotefulDocument` to formatted JSON.
    pub fn encode_json(doc: &NotefulDocument) -> Result<String> {
        serde_json::to_string_pretty(doc).map_err(|e| Error::InvalidEvent(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drawing::{NotefulTape, RgbaColor, StrokePoint, StrokeToolKind, VectorStroke};

    #[test]
    fn test_binary_roundtrip() {
        let mut doc = NotefulDocument::new("Physics Mechanics");
        let page = &mut doc.pages[0];
        let layer = page.active_layer_mut().unwrap();

        let mut stroke = VectorStroke::new(StrokeToolKind::Pen, RgbaColor::BLUE, 3.0);
        stroke.points.push(StrokePoint::new(10.0, 10.0, 1.0, 100));
        stroke.points.push(StrokePoint::new(20.0, 25.0, 1.2, 110));
        layer.strokes.push(stroke);

        layer.tapes.push(NotefulTape::new(100.0, 50.0, 80.0, 20.0, RgbaColor::TAPE_YELLOW));

        let encoded = NotefulCodec::encode_binary(&doc).expect("Encoding binary should succeed");
        assert!(encoded.len() > 32);

        // Verify magic at start
        let magic_start = u32::from_be_bytes([encoded[0], encoded[1], encoded[2], encoded[3]]);
        assert_eq!(magic_start, NOTEFUL_MAGIC);

        // Verify magic at trailer
        let trailer_magic = u32::from_be_bytes([
            encoded[encoded.len() - 16],
            encoded[encoded.len() - 15],
            encoded[encoded.len() - 14],
            encoded[encoded.len() - 13],
        ]);
        assert_eq!(trailer_magic, NOTEFUL_MAGIC);

        // Decode roundtrip
        let decoded = NotefulCodec::decode(&encoded).expect("Decoding binary should succeed");
        assert_eq!(decoded.title, "Physics Mechanics");
        assert_eq!(decoded.pages[0].layers[0].strokes.len(), 1);
        assert_eq!(decoded.pages[0].layers[0].tapes.len(), 1);
        assert_eq!(decoded.pages[0].layers[0].tapes[0].width, 80.0);
    }
}
