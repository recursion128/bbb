use std::cell::RefCell;
use std::ops::Range;

use bytes::{Buf, BytesMut};
use flate2::{Compress, Compression, Decompress, FlushCompress, FlushDecompress, Status};

use crate::codec::{Decoder, Encoder, ProtocolError, Result};

thread_local! {
    static THREAD_PACKET_ENCODER: RefCell<PacketEncoder> = RefCell::new(PacketEncoder::new());
}

/// Borrows this thread's reusable [`PacketEncoder`] so one-shot and shared
/// broadcast callers reuse a single zlib `Compress` allocation and frame buffer.
pub fn with_thread_local_encoder<R>(f: impl FnOnce(&mut PacketEncoder) -> R) -> R {
    THREAD_PACKET_ENCODER.with(|encoder| f(&mut encoder.borrow_mut()))
}

/// Upper bound for one decompressed packet body (packet id plus payload).
const MAX_LOGICAL_PACKET_SIZE: usize = 8 * 1024 * 1024;
/// `Varint21FrameDecoder` and `Varint21LengthFieldPrepender` admit exactly a
/// three-byte outer length prefix (`Varint21FrameDecoder.java:11-34`,
/// `Varint21LengthFieldPrepender.java:10-24`).
const MAX_FRAME_SIZE: usize = 0x1f_ffff;
const MAX_OUTER_VARINT_LEN: usize = 3;
const MAX_RETAINED_SCRATCH_CAPACITY: usize = 256 * 1024;
const SCRATCH_SHRINK_THRESHOLD: usize = 64 * 1024;
const INITIAL_SCRATCH_CAPACITY: usize = 8 * 1024;

pub fn encode_packet(packet_id: i32, payload: &[u8]) -> Vec<u8> {
    encode_packet_with_compression(packet_id, payload, None)
        .expect("trusted packet must fit the protocol frame limit")
}

pub fn encode_packet_with_compression(
    packet_id: i32,
    payload: &[u8],
    compression_threshold: Option<i32>,
) -> Result<Vec<u8>> {
    let (id_bytes, id_len) = encode_var_i32_stack(packet_id);
    let id_slice = &id_bytes[..id_len];
    let body_len = checked_logical_body_len(id_len, payload.len())?;

    let Some(threshold) = compression_threshold.filter(|value| *value >= 0) else {
        ensure_frame_size(body_len)?;
        let (outer_bytes, outer_len) = encode_var_i32_stack(body_len as i32);
        let mut packet = Vec::with_capacity(outer_len + body_len);
        packet.extend_from_slice(&outer_bytes[..outer_len]);
        packet.extend_from_slice(id_slice);
        packet.extend_from_slice(payload);
        return Ok(packet);
    };

    if body_len < threshold as usize {
        let framed_len = body_len
            .checked_add(1)
            .ok_or(ProtocolError::PacketTooLarge(usize::MAX, MAX_FRAME_SIZE))?;
        ensure_frame_size(framed_len)?;
        let (outer_bytes, outer_len) = encode_var_i32_stack(framed_len as i32);
        let mut packet = Vec::with_capacity(outer_len + framed_len);
        packet.extend_from_slice(&outer_bytes[..outer_len]);
        packet.push(0);
        packet.extend_from_slice(id_slice);
        packet.extend_from_slice(payload);
        return Ok(packet);
    }

    with_thread_local_encoder(|encoder| {
        encoder
            .encode(packet_id, payload, compression_threshold)
            .map(|framed| framed.to_vec())
    })
}

/// Connection-owned storage for encoding successive wire frames with a
/// reusable zlib deflater and output scratch buffer.
#[derive(Default)]
pub struct PacketEncoder {
    deflater: Option<Compress>,
    frame: Vec<u8>,
}

impl PacketEncoder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn encode(
        &mut self,
        packet_id: i32,
        payload: &[u8],
        compression_threshold: Option<i32>,
    ) -> Result<&[u8]> {
        let (id_bytes, id_len) = encode_var_i32_stack(packet_id);
        let id_slice = &id_bytes[..id_len];
        let body_len = checked_logical_body_len(id_len, payload.len())?;

        let Some(threshold) = compression_threshold.filter(|value| *value >= 0) else {
            ensure_frame_size(body_len)?;
            let (outer_bytes, outer_len) = encode_var_i32_stack(body_len as i32);
            let total_len = outer_len + body_len;
            prepare_scratch_buffer(&mut self.frame, total_len);
            self.frame.extend_from_slice(&outer_bytes[..outer_len]);
            self.frame.extend_from_slice(id_slice);
            self.frame.extend_from_slice(payload);
            return Ok(&self.frame);
        };

        if body_len < threshold as usize {
            let framed_len = body_len
                .checked_add(1)
                .ok_or(ProtocolError::PacketTooLarge(usize::MAX, MAX_FRAME_SIZE))?;
            ensure_frame_size(framed_len)?;
            let (outer_bytes, outer_len) = encode_var_i32_stack(framed_len as i32);
            let total_len = outer_len + framed_len;
            prepare_scratch_buffer(&mut self.frame, total_len);
            self.frame.extend_from_slice(&outer_bytes[..outer_len]);
            self.frame.push(0);
            self.frame.extend_from_slice(id_slice);
            self.frame.extend_from_slice(payload);
            return Ok(&self.frame);
        }

        let estimated = MAX_OUTER_VARINT_LEN + 5 + (body_len / 2).clamp(64, 64 * 1024);
        prepare_scratch_buffer(&mut self.frame, estimated);
        let deflater = match &mut self.deflater {
            Some(deflater) => {
                deflater.reset();
                deflater
            }
            None => self
                .deflater
                .insert(Compress::new(Compression::fast(), true)),
        };
        match encode_compressed_into(id_slice, payload, body_len, deflater, &mut self.frame) {
            Ok(start) => Ok(&self.frame[start..]),
            Err(error) => {
                trim_oversized_scratch(&mut self.frame);
                Err(error)
            }
        }
    }
}

fn checked_logical_body_len(id_len: usize, payload_len: usize) -> Result<usize> {
    let body_len = id_len
        .checked_add(payload_len)
        .ok_or(ProtocolError::PacketTooLarge(
            usize::MAX,
            MAX_LOGICAL_PACKET_SIZE,
        ))?;
    ensure_logical_packet_size(body_len)?;
    Ok(body_len)
}

fn encode_var_i32_stack(value: i32) -> ([u8; 5], usize) {
    let mut bytes = [0_u8; 5];
    let mut current = value as u32;
    let mut len = 0;
    loop {
        if (current & !0x7f) == 0 {
            bytes[len] = current as u8;
            return (bytes, len + 1);
        }
        bytes[len] = ((current & 0x7f) as u8) | 0x80;
        len += 1;
        current >>= 7;
    }
}

fn prepare_scratch_buffer(buffer: &mut Vec<u8>, needed: usize) {
    if buffer.capacity() > MAX_RETAINED_SCRATCH_CAPACITY && needed <= SCRATCH_SHRINK_THRESHOLD {
        *buffer = Vec::with_capacity(needed.max(INITIAL_SCRATCH_CAPACITY));
    } else {
        buffer.clear();
        buffer.reserve(needed);
    }
}

fn trim_oversized_scratch(buffer: &mut Vec<u8>) {
    if buffer.capacity() > MAX_RETAINED_SCRATCH_CAPACITY {
        *buffer = Vec::with_capacity(INITIAL_SCRATCH_CAPACITY);
    } else {
        buffer.clear();
    }
}

fn encode_compressed_into(
    id_slice: &[u8],
    payload: &[u8],
    body_len: usize,
    deflater: &mut Compress,
    frame: &mut Vec<u8>,
) -> Result<usize> {
    let (data_len_bytes, data_len_len) = encode_var_i32_stack(body_len as i32);
    frame.extend_from_slice(&[0_u8; MAX_OUTER_VARINT_LEN]);
    frame.extend_from_slice(&data_len_bytes[..data_len_len]);

    if payload.is_empty() {
        compress_chunk(deflater, id_slice, FlushCompress::Finish, frame)?;
    } else {
        compress_chunk(deflater, id_slice, FlushCompress::None, frame)?;
        compress_chunk(deflater, payload, FlushCompress::Finish, frame)?;
    }

    let framed_len = frame.len() - MAX_OUTER_VARINT_LEN;
    ensure_frame_size(framed_len)?;
    let (outer_bytes, outer_len) = encode_var_i32_stack(framed_len as i32);
    let start = MAX_OUTER_VARINT_LEN - outer_len;
    frame[start..MAX_OUTER_VARINT_LEN].copy_from_slice(&outer_bytes[..outer_len]);
    Ok(start)
}

fn compress_chunk(
    deflater: &mut Compress,
    mut remaining: &[u8],
    flush: FlushCompress,
    frame: &mut Vec<u8>,
) -> Result<()> {
    const MAX_STEP_INPUT: usize = 64 * 1024;
    loop {
        let step_len = remaining.len().min(MAX_STEP_INPUT);
        let step_input = &remaining[..step_len];
        let step_flush = if step_len == remaining.len() {
            flush
        } else {
            FlushCompress::None
        };
        if frame.capacity() - frame.len() < 128 {
            frame.reserve((step_len / 2).clamp(128, 32 * 1024));
        }
        let in_before = deflater.total_in();
        let out_before = deflater.total_out();
        let status = deflater
            .compress_vec(step_input, frame, step_flush)
            .map_err(|err| ProtocolError::Zlib(std::io::Error::other(err)))?;
        let consumed = (deflater.total_in() - in_before) as usize;
        let produced = (deflater.total_out() - out_before) as usize;
        remaining = &remaining[consumed..];

        let framed_len = frame.len() - MAX_OUTER_VARINT_LEN;
        if framed_len > MAX_FRAME_SIZE {
            return Err(ProtocolError::PacketTooLarge(framed_len, MAX_FRAME_SIZE));
        }
        if remaining.is_empty()
            && (step_flush != FlushCompress::Finish || status == Status::StreamEnd)
        {
            return Ok(());
        }
        if consumed == 0 && produced == 0 {
            frame.reserve((frame.len() / 2).clamp(256, 64 * 1024));
        }
    }
}

fn ensure_logical_packet_size(len: usize) -> Result<()> {
    if len > MAX_LOGICAL_PACKET_SIZE {
        Err(ProtocolError::PacketTooLarge(len, MAX_LOGICAL_PACKET_SIZE))
    } else {
        Ok(())
    }
}

fn ensure_frame_size(len: usize) -> Result<()> {
    if len > MAX_FRAME_SIZE {
        Err(ProtocolError::PacketTooLarge(len, MAX_FRAME_SIZE))
    } else {
        Ok(())
    }
}

fn complete_frame(buffer: &[u8]) -> Result<Option<Range<usize>>> {
    let Some((packet_len, prefix_len)) = peek_var_i32(buffer)? else {
        return Ok(None);
    };
    if packet_len < 0 {
        return Err(ProtocolError::NegativeLength(packet_len));
    }
    let packet_len = packet_len as usize;
    if packet_len == 0 {
        return Err(ProtocolError::InvalidData(
            "frame length cannot be zero".to_string(),
        ));
    }
    if packet_len > MAX_FRAME_SIZE {
        return Err(ProtocolError::PacketTooLarge(packet_len, MAX_FRAME_SIZE));
    }
    if buffer.len() < prefix_len + packet_len {
        return Ok(None);
    }

    Ok(Some(prefix_len..prefix_len + packet_len))
}

pub fn try_read_frame(buffer: &mut BytesMut) -> Result<Option<Vec<u8>>> {
    let Some(frame) = complete_frame(buffer)? else {
        return Ok(None);
    };
    let data = buffer[frame.clone()].to_vec();
    buffer.advance(frame.end);
    Ok(Some(data))
}

pub fn decode_packet_body(frame: &[u8], compression_threshold: Option<i32>) -> Result<Vec<u8>> {
    decode_packet_body_validated(frame, compression_threshold, false)
}

pub fn decode_packet_body_validated(
    frame: &[u8],
    compression_threshold: Option<i32>,
    reject_under_threshold: bool,
) -> Result<Vec<u8>> {
    let mut body = Vec::new();
    decode_packet_body_into_validated(
        frame,
        compression_threshold,
        &mut None,
        &mut body,
        reject_under_threshold,
    )?;
    Ok(body)
}

fn decode_packet_body_into(
    frame: &[u8],
    compression_threshold: Option<i32>,
    inflater: &mut Option<Decompress>,
    body: &mut Vec<u8>,
) -> Result<()> {
    decode_packet_body_into_validated(frame, compression_threshold, inflater, body, false)
}

pub fn decode_packet_body_into_validated(
    frame: &[u8],
    compression_threshold: Option<i32>,
    inflater: &mut Option<Decompress>,
    body: &mut Vec<u8>,
    reject_under_threshold: bool,
) -> Result<()> {
    let Some(threshold) = compression_threshold.filter(|value| *value >= 0) else {
        prepare_scratch_buffer(body, frame.len());
        body.extend_from_slice(frame);
        return Ok(());
    };

    let mut decoder = Decoder::new(frame);
    let data_len = decoder.read_len()?;
    if data_len == 0 {
        let remaining = decoder.remaining();
        prepare_scratch_buffer(body, remaining.len());
        body.extend_from_slice(remaining);
        return Ok(());
    }
    // CompressionDecoder.java:31-34: serverbound validates non-zero lengths against `threshold`.
    if reject_under_threshold && data_len < threshold as usize {
        return Err(ProtocolError::InvalidData(format!(
            "badly compressed packet: size {data_len} is below compression threshold {threshold}"
        )));
    }
    if data_len > MAX_LOGICAL_PACKET_SIZE {
        return Err(ProtocolError::PacketTooLarge(
            data_len,
            MAX_LOGICAL_PACKET_SIZE,
        ));
    }

    let compressed = decoder.remaining();
    let max_output = data_len + 1;
    let initial_cap = data_len
        .min(compressed.len().saturating_mul(4))
        .clamp(1024, 65_536)
        .min(max_output);
    if body.capacity() > MAX_RETAINED_SCRATCH_CAPACITY && max_output <= SCRATCH_SHRINK_THRESHOLD {
        *body = Vec::with_capacity(initial_cap.max(INITIAL_SCRATCH_CAPACITY));
    } else {
        body.clear();
        if body.capacity() < initial_cap {
            body.reserve(initial_cap);
        }
    }
    let inflater = match inflater {
        Some(inflater) => {
            inflater.reset(true);
            inflater
        }
        None => inflater.insert(Decompress::new(true)),
    };
    let result = (|| {
        let mut written = 0usize;
        loop {
            let in_offset = inflater.total_in() as usize;
            let remaining_budget = max_output - written;
            if remaining_budget == 0 {
                return Err(ProtocolError::BadCompressionLength {
                    declared: data_len,
                    actual: written,
                });
            }
            if body.capacity() == written {
                let grow = written
                    .max(initial_cap)
                    .clamp(4 * 1024, 64 * 1024)
                    .min(remaining_budget);
                body.reserve_exact(grow);
            }
            let before_out = inflater.total_out();
            let before_in = inflater.total_in();
            let status = if body.capacity() <= max_output {
                inflater
                    .decompress_vec(&compressed[in_offset..], body, FlushDecompress::Sync)
                    .map_err(|err| ProtocolError::Zlib(err.into()))?
            } else {
                let target = written + remaining_budget.min(64 * 1024);
                body.resize(target, 0);
                let status = inflater
                    .decompress(
                        &compressed[in_offset..],
                        &mut body[written..target],
                        FlushDecompress::Sync,
                    )
                    .map_err(|err| ProtocolError::Zlib(err.into()))?;
                let produced = (inflater.total_out() - before_out) as usize;
                body.truncate(written + produced);
                status
            };
            let produced = (inflater.total_out() - before_out) as usize;
            written += produced;
            debug_assert_eq!(body.len(), written);
            if written > data_len {
                return Err(ProtocolError::BadCompressionLength {
                    declared: data_len,
                    actual: written,
                });
            }
            match status {
                Status::StreamEnd => {
                    if written != data_len {
                        return Err(ProtocolError::BadCompressionLength {
                            declared: data_len,
                            actual: written,
                        });
                    }
                    return Ok(());
                }
                Status::Ok | Status::BufError => {
                    if produced == 0 && inflater.total_in() == before_in {
                        if written != data_len {
                            return Err(ProtocolError::BadCompressionLength {
                                declared: data_len,
                                actual: written,
                            });
                        }
                        return Err(ProtocolError::InvalidData("incomplete zlib packet".into()));
                    }
                }
            }
        }
    })();
    if result.is_err() {
        trim_oversized_scratch(body);
    }
    result
}

/// Connection-owned storage for decoding successive wire frames.
#[derive(Default)]
pub struct PacketDecoder {
    inflater: Option<Decompress>,
    body: Vec<u8>,
    payload_start: usize,
}

impl PacketDecoder {
    pub fn try_read(
        &mut self,
        buffer: &mut BytesMut,
        compression_threshold: Option<i32>,
    ) -> Result<Option<i32>> {
        self.try_read_validated(buffer, compression_threshold, false)
    }

    pub fn try_read_validated(
        &mut self,
        buffer: &mut BytesMut,
        compression_threshold: Option<i32>,
        reject_under_threshold: bool,
    ) -> Result<Option<i32>> {
        let Some(frame) = complete_frame(buffer)? else {
            return Ok(None);
        };
        if reject_under_threshold {
            decode_packet_body_into_validated(
                &buffer[frame.clone()],
                compression_threshold,
                &mut self.inflater,
                &mut self.body,
                true,
            )?;
        } else {
            decode_packet_body_into(
                &buffer[frame.clone()],
                compression_threshold,
                &mut self.inflater,
                &mut self.body,
            )?;
        }
        let (id, payload) = split_packet(&self.body)?;
        self.payload_start = self.body.len() - payload.len();
        buffer.advance(frame.end);
        Ok(Some(id))
    }

    pub fn payload(&self) -> &[u8] {
        &self.body[self.payload_start..]
    }
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
    for position in 0..3 {
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
    use std::io::Write;

    use flate2::write::ZlibEncoder;

    use crate::codec::write_var_i32_to;

    use super::*;

    #[test]
    fn stateful_encoder_reuses_deflater_and_matches_stateless_framing() {
        let mut encoder = PacketEncoder::new();
        let mut decoder = PacketDecoder::default();
        let cases = [
            (0, vec![], None),
            (3, vec![1, 2, 3], Some(-1)),
            (19, vec![5; 12], Some(64)),
            (44, vec![7; 256], Some(16)),
            (
                302,
                (0..8192).map(|index| (index * 31) as u8).collect(),
                Some(64),
            ),
            (7, vec![], Some(0)),
            (128, vec![0xaa; 1024], None),
        ];

        for (id, payload, threshold) in &cases {
            let expected = encode_packet_with_compression(*id, payload, *threshold).unwrap();
            let actual = encoder.encode(*id, payload, *threshold).unwrap().to_vec();
            assert_eq!(actual, expected);

            let mut wire = BytesMut::from(actual.as_slice());
            assert_eq!(decoder.try_read(&mut wire, *threshold).unwrap(), Some(*id));
            assert_eq!(decoder.payload(), payload.as_slice());
            assert!(wire.is_empty());
        }

        // Verify scratch buffers shrink back down after a multi-megabyte burst.
        let large = vec![0_u8; MAX_LOGICAL_PACKET_SIZE - 1];
        let large_wire = encoder.encode(0, &large, Some(0)).unwrap().to_vec();
        let mut wire = BytesMut::from(large_wire.as_slice());
        assert_eq!(decoder.try_read(&mut wire, Some(0)).unwrap(), Some(0));
        assert_eq!(decoder.payload().len(), large.len());

        let small_wire = encoder.encode(1, &[42; 8], Some(64)).unwrap().to_vec();
        let mut wire = BytesMut::from(small_wire.as_slice());
        assert_eq!(decoder.try_read(&mut wire, Some(64)).unwrap(), Some(1));
        assert_eq!(decoder.payload(), &[42; 8]);
        assert!(decoder.body.capacity() <= MAX_RETAINED_SCRATCH_CAPACITY);
    }

    #[test]
    fn connection_decoder_handles_fragmented_and_coalesced_compressed_frames() {
        let threshold = Some(64);
        let packets = [(302, vec![17; 4096]), (3, vec![9]), (153, vec![28; 96])];
        let wire: Vec<_> = packets
            .iter()
            .flat_map(|(id, payload)| {
                encode_packet_with_compression(*id, payload, threshold).unwrap()
            })
            .collect();
        let mut decoder = PacketDecoder::default();
        let mut buffer = BytesMut::from(&wire[..3]);
        assert_eq!(decoder.try_read(&mut buffer, threshold).unwrap(), None);
        buffer.extend_from_slice(&wire[3..]);
        for (id, payload) in packets {
            assert_eq!(decoder.try_read(&mut buffer, threshold).unwrap(), Some(id));
            assert_eq!(decoder.payload(), payload);
        }
        assert!(buffer.is_empty());
    }

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

    #[test]
    fn negative_compression_threshold_disables_framing() {
        let uncompressed = encode_packet(3, &[1, 2, 3]);
        assert_eq!(
            encode_packet_with_compression(3, &[1, 2, 3], Some(-1)).unwrap(),
            uncompressed
        );

        let mut buffer = BytesMut::from(uncompressed.as_slice());
        let frame = try_read_frame(&mut buffer).unwrap().unwrap();
        assert_eq!(decode_packet_body(&frame, Some(-1)).unwrap(), frame);
    }

    #[test]
    fn encoder_accepts_the_maximum_logical_body_and_rejects_one_byte_more() {
        let boundary_payload = vec![0; MAX_LOGICAL_PACKET_SIZE - 1];
        let packet = encode_packet_with_compression(0, &boundary_payload, Some(0)).unwrap();
        let mut buffer = BytesMut::from(packet.as_slice());
        let frame = try_read_frame(&mut buffer).unwrap().unwrap();
        let body = decode_packet_body(&frame, Some(0)).unwrap();
        assert_eq!(body.len(), MAX_LOGICAL_PACKET_SIZE);

        let oversized_payload = vec![0; MAX_LOGICAL_PACKET_SIZE];
        let error = encode_packet_with_compression(0, &oversized_payload, None).unwrap_err();
        assert!(matches!(
            error,
            ProtocolError::PacketTooLarge(size, MAX_LOGICAL_PACKET_SIZE)
                if size == MAX_LOGICAL_PACKET_SIZE + 1
        ));
    }

    #[test]
    fn encoder_bounds_the_uncompressed_frame_body() {
        let boundary_payload = vec![0; MAX_FRAME_SIZE - 1];
        let packet = encode_packet_with_compression(0, &boundary_payload, None).unwrap();
        let mut buffer = BytesMut::from(packet.as_slice());
        let frame = try_read_frame(&mut buffer).unwrap().unwrap();
        assert_eq!(frame.len(), MAX_FRAME_SIZE);

        let oversized_payload = vec![0; MAX_FRAME_SIZE];
        let error = encode_packet_with_compression(0, &oversized_payload, None).unwrap_err();
        assert!(matches!(
            error,
            ProtocolError::PacketTooLarge(size, MAX_FRAME_SIZE)
                if size == MAX_FRAME_SIZE + 1
        ));
    }

    #[test]
    fn encoder_bounds_the_compression_framed_body() {
        let boundary_payload = vec![0; MAX_FRAME_SIZE - 2];
        let packet = encode_packet_with_compression(0, &boundary_payload, Some(i32::MAX)).unwrap();
        let mut buffer = BytesMut::from(packet.as_slice());
        let frame = try_read_frame(&mut buffer).unwrap().unwrap();
        assert_eq!(frame.len(), MAX_FRAME_SIZE);

        let oversized_payload = vec![0; MAX_FRAME_SIZE - 1];
        let error =
            encode_packet_with_compression(0, &oversized_payload, Some(i32::MAX)).unwrap_err();
        assert!(matches!(
            error,
            ProtocolError::PacketTooLarge(size, MAX_FRAME_SIZE)
                if size == MAX_FRAME_SIZE + 1
        ));
    }

    #[test]
    fn encoder_rejects_compressed_output_larger_than_the_frame_limit() {
        let mut state = 0x1234_5678_u32;
        let mut payload = vec![0; MAX_LOGICAL_PACKET_SIZE - 1];
        for byte in &mut payload {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            *byte = state as u8;
        }

        let error = encode_packet_with_compression(0, &payload, Some(0)).unwrap_err();
        assert!(matches!(
            error,
            ProtocolError::PacketTooLarge(size, MAX_FRAME_SIZE) if size > MAX_FRAME_SIZE
        ));
    }

    #[test]
    fn frame_reader_rejects_zero_and_wider_than_varint21_lengths() {
        let mut zero = BytesMut::from(&[0_u8][..]);
        assert!(matches!(
            try_read_frame(&mut zero),
            Err(ProtocolError::InvalidData(message))
                if message == "frame length cannot be zero"
        ));

        let mut too_wide = BytesMut::from(&[0x80_u8, 0x80, 0x80, 0x01][..]);
        assert!(matches!(
            try_read_frame(&mut too_wide),
            Err(ProtocolError::VarIntTooLong)
        ));
    }

    #[test]
    fn compressed_frame_rejects_oversized_declared_output_before_inflate() {
        let mut frame = Vec::new();
        write_var_i32_to(&mut frame, (MAX_LOGICAL_PACKET_SIZE + 1) as i32);
        frame.push(0);

        let error = decode_packet_body(&frame, Some(256)).unwrap_err();
        assert!(matches!(
            error,
            ProtocolError::PacketTooLarge(size, MAX_LOGICAL_PACKET_SIZE)
                if size == MAX_LOGICAL_PACKET_SIZE + 1
        ));
    }

    #[test]
    fn compressed_frame_bounds_output_to_declared_length() {
        let mut compressed = ZlibEncoder::new(Vec::new(), Compression::fast());
        compressed.write_all(&[7; 32]).unwrap();
        let compressed = compressed.finish().unwrap();
        let mut frame = Vec::new();
        write_var_i32_to(&mut frame, 16);
        frame.extend_from_slice(&compressed);

        let error = decode_packet_body(&frame, Some(1)).unwrap_err();
        assert!(matches!(
            error,
            ProtocolError::BadCompressionLength {
                declared: 16,
                actual: 17
            }
        ));
    }

    #[test]
    fn sub_threshold_compressed_frame_is_rejected_only_when_validation_enabled() {
        let raw_body = vec![0x2a; 32];
        let mut compressed = ZlibEncoder::new(Vec::new(), Compression::fast());
        compressed.write_all(&raw_body).unwrap();
        let compressed = compressed.finish().unwrap();

        let mut frame = Vec::new();
        write_var_i32_to(&mut frame, raw_body.len() as i32);
        frame.extend_from_slice(&compressed);

        // Clientbound default accepts valid zlib packets declared under threshold.
        assert_eq!(decode_packet_body(&frame, Some(256)).unwrap(), raw_body);

        // Serverbound validation rejects non-zero declared lengths below threshold.
        let error = decode_packet_body_validated(&frame, Some(256), true).unwrap_err();
        assert!(matches!(
            error,
            ProtocolError::InvalidData(message)
                if message.contains("below compression threshold 256")
        ));

        // Uncompressed framing (`data_len == 0`) bypasses threshold validation even when validated.
        let mut uncompressed_frame = Vec::new();
        write_var_i32_to(&mut uncompressed_frame, 0);
        uncompressed_frame.extend_from_slice(&raw_body);
        assert_eq!(
            decode_packet_body_validated(&uncompressed_frame, Some(256), true).unwrap(),
            raw_body
        );
    }
}
