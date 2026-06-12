use std::io::{Read, Write};

use bytes::{Buf, BytesMut};
use flate2::{read::ZlibDecoder, write::ZlibEncoder, Compression};

use crate::codec::{write_var_i32_to, Decoder, Encoder, ProtocolError, Result};

const MAX_PACKET_SIZE: usize = 8 * 1024 * 1024;

pub fn encode_packet(packet_id: i32, payload: &[u8]) -> Vec<u8> {
    encode_packet_with_compression(packet_id, payload, None)
        .expect("uncompressed encode cannot fail")
}

pub fn encode_packet_with_compression(
    packet_id: i32,
    payload: &[u8],
    compression_threshold: Option<i32>,
) -> Result<Vec<u8>> {
    let mut body = Vec::new();
    write_var_i32_to(&mut body, packet_id);
    body.extend_from_slice(payload);

    let framed_body = if let Some(threshold) = compression_threshold {
        if threshold >= 0 && body.len() >= threshold as usize {
            let mut encoder = ZlibEncoder::new(Vec::new(), Compression::fast());
            encoder.write_all(&body).map_err(ProtocolError::Zlib)?;
            let compressed = encoder.finish().map_err(ProtocolError::Zlib)?;

            let mut compressed_body = Vec::new();
            write_var_i32_to(&mut compressed_body, body.len() as i32);
            compressed_body.extend_from_slice(&compressed);
            compressed_body
        } else {
            let mut uncompressed_body = Vec::new();
            write_var_i32_to(&mut uncompressed_body, 0);
            uncompressed_body.extend_from_slice(&body);
            uncompressed_body
        }
    } else {
        body
    };

    let mut packet = Vec::new();
    write_var_i32_to(&mut packet, framed_body.len() as i32);
    packet.extend_from_slice(&framed_body);
    Ok(packet)
}

pub fn try_read_frame(buffer: &mut BytesMut) -> Result<Option<Vec<u8>>> {
    let Some((packet_len, prefix_len)) = peek_var_i32(buffer)? else {
        return Ok(None);
    };
    if packet_len < 0 {
        return Err(ProtocolError::NegativeLength(packet_len));
    }
    let packet_len = packet_len as usize;
    if packet_len > MAX_PACKET_SIZE {
        return Err(ProtocolError::PacketTooLarge(packet_len, MAX_PACKET_SIZE));
    }
    if buffer.len() < prefix_len + packet_len {
        return Ok(None);
    }

    buffer.advance(prefix_len);
    Ok(Some(buffer.split_to(packet_len).to_vec()))
}

pub fn decode_packet_body(frame: &[u8], compression_threshold: Option<i32>) -> Result<Vec<u8>> {
    let Some(_) = compression_threshold else {
        return Ok(frame.to_vec());
    };

    let mut decoder = Decoder::new(frame);
    let data_len = decoder.read_len()?;
    if data_len == 0 {
        return Ok(decoder.remaining().to_vec());
    }

    let mut zlib = ZlibDecoder::new(decoder.remaining());
    let mut decoded = Vec::with_capacity(data_len);
    zlib.read_to_end(&mut decoded)
        .map_err(ProtocolError::Zlib)?;
    if decoded.len() != data_len {
        return Err(ProtocolError::BadCompressionLength {
            declared: data_len,
            actual: decoded.len(),
        });
    }
    Ok(decoded)
}

pub fn split_packet(data: &[u8]) -> Result<(i32, &[u8])> {
    let mut decoder = Decoder::new(data);
    let packet_id = decoder.read_var_i32()?;
    Ok((packet_id, decoder.remaining()))
}

pub fn empty_payload() -> Vec<u8> {
    Encoder::new().into_inner()
}

fn peek_var_i32(bytes: &[u8]) -> Result<Option<(i32, usize)>> {
    let mut value = 0u32;
    for position in 0..5 {
        let Some(byte) = bytes.get(position).copied() else {
            return Ok(None);
        };
        value |= ((byte & 0x7f) as u32) << (7 * position);
        if byte & 0x80 == 0 {
            return Ok(Some((value as i32, position + 1)));
        }
    }
    Err(ProtocolError::VarIntTooLong)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_roundtrip_uncompressed() {
        let packet = encode_packet(3, &[1, 2, 3]);
        let mut buf = BytesMut::from(packet.as_slice());
        let frame = try_read_frame(&mut buf).unwrap().unwrap();
        let (id, payload) = split_packet(&frame).unwrap();
        assert_eq!(id, 3);
        assert_eq!(payload, &[1, 2, 3]);
    }

    #[test]
    fn frame_roundtrip_compressed() {
        let packet = encode_packet_with_compression(44, &[7; 256], Some(16)).unwrap();
        let mut buf = BytesMut::from(packet.as_slice());
        let frame = try_read_frame(&mut buf).unwrap().unwrap();
        let body = decode_packet_body(&frame, Some(16)).unwrap();
        let (id, payload) = split_packet(&body).unwrap();
        assert_eq!(id, 44);
        assert_eq!(payload, &[7; 256]);
    }
}
