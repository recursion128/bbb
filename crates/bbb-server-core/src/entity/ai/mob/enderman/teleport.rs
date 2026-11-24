use bbb_protocol::packets::Vec3d;

#[derive(Debug, Clone, Copy)]
pub(crate) struct Request {
    pub(crate) destination: Vec3d,
    pub(crate) reset_timer_on_success: bool,
}
