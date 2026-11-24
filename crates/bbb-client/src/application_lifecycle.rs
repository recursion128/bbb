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

    /// Release the window, renderer and surface the handler owns while the
    /// loop that created them is still alive. `run_app` takes the event loop
    /// by value, so a handler that survives it destroys its swapchain against
    /// a display connection that has already been closed.
    fn release_handler(&mut self) {
        self.handler = None;
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
        self.release_handler();
    }

    fn memory_warning(&mut self, event_loop: &ActiveEventLoop) {
        self.dispatch(Event::MemoryWarning, event_loop);
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::Cell, rc::Rc};

    use winit::{event::Event, event_loop::ActiveEventLoop};

    use super::{NativeApplication, NativeEventHandler};

    /// The renderer and its surface live inside the handler. Nothing may hold
    /// them once the loop that owns the display connection is gone, so the
    /// release has to actually drop what the handler captured.
    #[test]
    fn releasing_the_handler_drops_what_the_window_owns() {
        struct Captured(Rc<Cell<bool>>);
        impl Drop for Captured {
            fn drop(&mut self) {
                self.0.set(true);
            }
        }

        let dropped = Rc::new(Cell::new(false));
        let captured = Captured(Rc::clone(&dropped));
        let mut application =
            NativeApplication::new(|_| unreachable!("this application never bootstraps"));
        let handler: NativeEventHandler =
            Box::new(move |_event: Event<()>, _event_loop: &ActiveEventLoop| {
                let _ = &captured;
            });
        application.handler = Some(handler);

        assert!(!dropped.get());
        application.release_handler();
        assert!(dropped.get(), "the handler still owns the renderer");
        assert!(application.handler.is_none());
    }
}
