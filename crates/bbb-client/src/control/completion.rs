use bbb_client_render::CapturedScreenshot;
use tokio::sync::oneshot;

#[derive(Debug)]
pub enum ControlCompletion {
    Recording(super::RecordingStatus),
    Processed {
        generation: u64,
    },
    Screenshot {
        path: String,
        image: CapturedScreenshot,
    },
}

pub type ControlCompletionSender = oneshot::Sender<Result<ControlCompletion, String>>;
pub type ControlCompletionReceiver = oneshot::Receiver<Result<ControlCompletion, String>>;
