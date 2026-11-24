use anyhow::Result;
use bbb_protocol::packets::LoginClientbound;

use crate::{
    event_stream::{emit, EventStreamContext},
    session::LoginPacketStep,
    types::NetEvent,
};

impl EventStreamContext {
    pub(super) async fn handle_login_packet(&mut self, packet: LoginClientbound) -> Result<()> {
        match self.session.step_login_packet(packet).await? {
            LoginPacketStep::CompressionSet { threshold } => {
                emit(&self.events, NetEvent::CompressionSet { threshold }).await?;
            }
            LoginPacketStep::CookieRequest {
                key,
                response_payload_present,
            } => {
                emit(
                    &self.events,
                    NetEvent::CookieRequest {
                        key,
                        response_payload_present,
                    },
                )
                .await?;
            }
            LoginPacketStep::EnteredConfiguration => {
                emit(&self.events, NetEvent::StateChanged { state: self.state }).await?;
            }
            LoginPacketStep::Unknown { packet_id, len } => {
                emit(
                    &self.events,
                    NetEvent::UnsupportedPacket {
                        state: self.state,
                        packet_id,
                        len,
                    },
                )
                .await?;
            }
            LoginPacketStep::Handled => {}
        }
        Ok(())
    }
}
