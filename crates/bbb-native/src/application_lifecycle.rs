//! Narrow winit 0.30 application-lifecycle adapter.
//!
//! Native client assembly still has one owner in `run_client`, but window and
//! surface creation must happen from the first `ApplicationHandler::resumed`
//! callback. After that bootstrap, every typed callback is projected back into
//! the single ordered `Event` stream consumed by the existing runtime.

use anyhow::Result;
use winit::{
    application::ApplicationHandler,
    event::{DeviceEvent, DeviceId, Event, StartCause, WindowEvent},
    event_loop::ActiveEventLoop,
    window::WindowId,
};

pub(crate) type NativeEventHandler = Box<dyn FnMut(Event<()>, &ActiveEventLoop)>;
type NativeBootstrap = Box<dyn FnOnce(&ActiveEventLoop) -> Result<NativeEventHandler>>;

pub(crate) struct NativeApplication {
    bootstrap: Option<NativeBootstrap>,
    handler: Option<NativeEventHandler>,
    pending_start_cause: Option<StartCause>,
    bootstrap_error: Option<anyhow::Error>,
}

impl NativeApplication {
    pub(crate) fn new(
        bootstrap: impl FnOnce(&ActiveEventLoop) -> Result<NativeEventHandler> + 'static,
    ) -> Self {
        Self {
            bootstrap: Some(Box::new(bootstrap)),
            handler: None,
            pending_start_cause: None,
            bootstrap_error: None,
        }
    }

    pub(crate) fn take_bootstrap_error(&mut self) -> Option<anyhow::Error> {
        self.bootstrap_error.take()
    }

    fn dispatch(&mut self, event: Event<()>, event_loop: &ActiveEventLoop) {
        if let Some(handler) = self.handler.as_mut() {
            handler(event, event_loop);
        }
    }
}

impl ApplicationHandler<()> for NativeApplication {
    fn new_events(&mut self, event_loop: &ActiveEventLoop, cause: StartCause) {
        if self.handler.is_some() {
            self.dispatch(Event::NewEvents(cause), event_loop);
        } else {
            self.pending_start_cause = Some(cause);
        }
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.handler.is_none() && self.bootstrap_error.is_none() {
            let Some(bootstrap) = self.bootstrap.take() else {
                self.bootstrap_error = Some(anyhow::anyhow!(
                    "native application bootstrap owner was consumed without a handler"
                ));
                event_loop.exit();
                return;
            };
            match bootstrap(event_loop) {
                Ok(handler) => self.handler = Some(handler),
                Err(error) => {
                    self.bootstrap_error = Some(error);
                    event_loop.exit();
                    return;
                }
            }
        }
        if let Some(cause) = self.pending_start_cause.take() {
            self.dispatch(Event::NewEvents(cause), event_loop);
        }
        self.dispatch(Event::Resumed, event_loop);
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: ()) {
        self.dispatch(Event::UserEvent(event), event_loop);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        self.dispatch(Event::WindowEvent { window_id, event }, event_loop);
    }

    fn device_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        device_id: DeviceId,
        event: DeviceEvent,
    ) {
        self.dispatch(Event::DeviceEvent { device_id, event }, event_loop);
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.dispatch(Event::AboutToWait, event_loop);
    }

    fn suspended(&mut self, event_loop: &ActiveEventLoop) {
        self.dispatch(Event::Suspended, event_loop);
    }

    fn exiting(&mut self, event_loop: &ActiveEventLoop) {
        self.dispatch(Event::LoopExiting, event_loop);
    }

    fn memory_warning(&mut self, event_loop: &ActiveEventLoop) {
        self.dispatch(Event::MemoryWarning, event_loop);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn adapter_covers_every_winit_application_callback_without_parallel_owners() {
        let source = include_str!("application_lifecycle.rs");
        let source = source
            .split_once("\n#[cfg(test)]")
            .expect("production/test boundary")
            .0;
        for projection in [
            "self.dispatch(Event::UserEvent(event), event_loop);",
            "self.dispatch(Event::WindowEvent { window_id, event }, event_loop);",
            "self.dispatch(Event::DeviceEvent { device_id, event }, event_loop);",
            "self.dispatch(Event::AboutToWait, event_loop);",
            "self.dispatch(Event::Suspended, event_loop);",
            "self.dispatch(Event::LoopExiting, event_loop);",
            "self.dispatch(Event::MemoryWarning, event_loop);",
        ] {
            assert_eq!(source.matches(projection).count(), 1, "{projection}");
        }
        assert_eq!(
            source
                .matches("self.dispatch(Event::Resumed, event_loop);")
                .count(),
            1
        );
        assert_eq!(
            source
                .matches("self.dispatch(Event::NewEvents(cause), event_loop);")
                .count(),
            2
        );
        assert_eq!(source.matches("bootstrap(event_loop)").count(), 1);
        assert!(source.contains("self.pending_start_cause = Some(cause)"));
        assert!(source.contains("event_loop.exit()"));
    }

    #[test]
    fn workspace_and_lock_resolve_only_winit_0_30_13() {
        let manifest = include_str!("../../../Cargo.toml");
        let lock = include_str!("../../../Cargo.lock");

        assert!(manifest.contains("winit = \"0.30.13\""));
        assert!(!manifest.contains("winit = \"0.29"));
        let package = "name = \"winit\"\nversion = \"0.30.13\"";
        assert_eq!(lock.matches(package).count(), 1);
        assert!(!lock.contains("name = \"winit\"\nversion = \"0.29"));
    }
}
