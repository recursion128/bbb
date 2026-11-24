use sha2::{Digest, Sha256};

#[derive(Clone, Debug)]
pub(super) struct ProtoEntityUuidSequence {
    world_seed: i64,
    center_x: i32,
    center_z: i32,
    next: u64,
}

impl ProtoEntityUuidSequence {
    pub(super) const fn new(world_seed: i64, center_x: i32, center_z: i32) -> Self {
        Self {
            world_seed,
            center_x,
            center_z,
            next: 0,
        }
    }

    pub(super) fn next_uuid(&mut self) -> [i32; 4] {
        let mut hash = Sha256::new();
        hash.update(b"bbb:worldgen-proto-entity-uuid:v1");
        hash.update(self.world_seed.to_be_bytes());
        hash.update(self.center_x.to_be_bytes());
        hash.update(self.center_z.to_be_bytes());
        hash.update(self.next.to_be_bytes());
        self.next = self.next.wrapping_add(1);
        let mut bytes: [u8; 16] = hash.finalize()[..16]
            .try_into()
            .expect("SHA-256 has at least 16 bytes");
        bytes[6] = (bytes[6] & 0x0f) | 0x40;
        bytes[8] = (bytes[8] & 0x3f) | 0x80;
        [
            i32::from_be_bytes(bytes[0..4].try_into().unwrap()),
            i32::from_be_bytes(bytes[4..8].try_into().unwrap()),
            i32::from_be_bytes(bytes[8..12].try_into().unwrap()),
            i32::from_be_bytes(bytes[12..16].try_into().unwrap()),
        ]
    }
}

#[cfg(test)]
mod tests {}
