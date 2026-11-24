use std::ops::Deref;

use anyhow::{bail, Result};
use bbb_protocol::packets::PlayClientbound;

const BUNDLE_SIZE_LIMIT: usize = 4096;

/// One dispatched play packet or completed delimiter bundle, avoiding a heap
/// `Vec` allocation for the common unbundled single-packet path.
#[derive(Debug, PartialEq)]
// Keep PlayClientbound inline so single packets avoid per-packet heap allocation.
#[allow(clippy::large_enum_variant)]
pub enum PlayPacketBatch {
    Single(PlayClientbound),
    Bundle(Vec<PlayClientbound>),
}

impl PlayPacketBatch {
    pub fn as_slice(&self) -> &[PlayClientbound] {
        match self {
            Self::Single(packet) => std::slice::from_ref(packet),
            Self::Bundle(packets) => packets.as_slice(),
        }
    }
}

impl Deref for PlayPacketBatch {
    type Target = [PlayClientbound];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

// Keep PlayClientbound inline so single packets avoid per-packet heap allocation.
#[allow(clippy::large_enum_variant)]
pub enum PlayPacketBatchIter {
    Single(std::option::IntoIter<PlayClientbound>),
    Bundle(std::vec::IntoIter<PlayClientbound>),
}

impl Iterator for PlayPacketBatchIter {
    type Item = PlayClientbound;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Single(iter) => iter.next(),
            Self::Bundle(iter) => iter.next(),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match self {
            Self::Single(iter) => iter.size_hint(),
            Self::Bundle(iter) => iter.size_hint(),
        }
    }
}

impl ExactSizeIterator for PlayPacketBatchIter {}

impl IntoIterator for PlayPacketBatch {
    type Item = PlayClientbound;
    type IntoIter = PlayPacketBatchIter;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            Self::Single(packet) => PlayPacketBatchIter::Single(Some(packet).into_iter()),
            Self::Bundle(packets) => PlayPacketBatchIter::Bundle(packets.into_iter()),
        }
    }
}

/// Connection-owned inbound play bundling, mirroring vanilla
/// `PacketBundlePacker`/`BundlerInfo`: delimiters never reach gameplay owners,
/// and a completed bundle is dispatched as one ordered batch.
#[derive(Debug, Default)]
pub struct PlayPacketBundleAssembler {
    pending: Option<Vec<PlayClientbound>>,
}

impl PlayPacketBundleAssembler {
    pub fn push(&mut self, packet: PlayClientbound) -> Result<Option<PlayPacketBatch>> {
        if self.pending.is_some() && is_terminal(&packet) {
            bail!("terminal play packet received inside bundle");
        }

        if matches!(packet, PlayClientbound::BundleDelimiter) {
            return if let Some(packets) = self.pending.take() {
                Ok(Some(PlayPacketBatch::Bundle(packets)))
            } else {
                self.pending = Some(Vec::new());
                Ok(None)
            };
        }

        let Some(pending) = self.pending.as_mut() else {
            return Ok(Some(PlayPacketBatch::Single(packet)));
        };
        if pending.len() >= BUNDLE_SIZE_LIMIT {
            bail!("too many packets in a play bundle");
        }
        pending.push(packet);
        Ok(None)
    }
}

fn is_terminal(packet: &PlayClientbound) -> bool {
    matches!(packet, PlayClientbound::StartConfiguration)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundle_holds_packets_until_closing_delimiter_and_preserves_order() {
        let mut assembler = PlayPacketBundleAssembler::default();
        assert!(assembler
            .push(PlayClientbound::BundleDelimiter)
            .unwrap()
            .is_none());
        assert!(assembler
            .push(PlayClientbound::KeepAlive { id: 7 })
            .unwrap()
            .is_none());
        assert!(assembler
            .push(PlayClientbound::Ping { id: 9 })
            .unwrap()
            .is_none());

        let batch = assembler
            .push(PlayClientbound::BundleDelimiter)
            .unwrap()
            .unwrap();
        assert_eq!(
            batch.as_slice(),
            &[
                PlayClientbound::KeepAlive { id: 7 },
                PlayClientbound::Ping { id: 9 }
            ]
        );
        assert_eq!(
            assembler
                .push(PlayClientbound::KeepAlive { id: 11 })
                .unwrap()
                .unwrap(),
            PlayPacketBatch::Single(PlayClientbound::KeepAlive { id: 11 })
        );
    }

    #[test]
    fn empty_bundle_dispatches_an_empty_atomic_batch() {
        let mut assembler = PlayPacketBundleAssembler::default();
        assert!(assembler
            .push(PlayClientbound::BundleDelimiter)
            .unwrap()
            .is_none());
        assert!(assembler
            .push(PlayClientbound::BundleDelimiter)
            .unwrap()
            .unwrap()
            .is_empty());
    }

    #[test]
    fn bundle_accepts_4096_packets_and_rejects_the_next() {
        let mut assembler = PlayPacketBundleAssembler::default();
        assembler.push(PlayClientbound::BundleDelimiter).unwrap();
        for packet_id in 0..BUNDLE_SIZE_LIMIT {
            assert!(assembler
                .push(PlayClientbound::Unknown {
                    packet_id: packet_id as i32,
                    len: 0,
                })
                .unwrap()
                .is_none());
        }
        let error = assembler
            .push(PlayClientbound::Unknown {
                packet_id: BUNDLE_SIZE_LIMIT as i32,
                len: 0,
            })
            .unwrap_err();
        assert!(error.to_string().contains("too many packets"));
    }

    #[test]
    fn terminal_packet_is_rejected_only_inside_bundle() {
        let mut assembler = PlayPacketBundleAssembler::default();
        assert_eq!(
            assembler
                .push(PlayClientbound::StartConfiguration)
                .unwrap()
                .unwrap(),
            PlayPacketBatch::Single(PlayClientbound::StartConfiguration)
        );
        assembler.push(PlayClientbound::BundleDelimiter).unwrap();
        let error = assembler
            .push(PlayClientbound::StartConfiguration)
            .unwrap_err();
        assert!(error.to_string().contains("terminal play packet"));
    }
}
