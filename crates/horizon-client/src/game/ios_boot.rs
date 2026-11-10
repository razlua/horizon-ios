//! iOS: the first start unpacks the prepared resources (entry.rs), which takes longer than the
//! system gives an app to start before it ends it. The event loop starts at once with nothing
//! on screen, the unpacking runs on a thread of its own, and the game starts inside the running
//! loop when it is done: `run_loading` hands its app over instead of opening a loop.

use super::App;
use std::cell::RefCell;
use std::time::{Duration, Instant};
use winit::{
    application::ApplicationHandler,
    event::{DeviceEvent, DeviceId, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::WindowId,
};

thread_local! {
    /// `Some` while the loop of `run_after` waits for the game's app, then the app itself.
    static HANDED: RefCell<Option<Option<App>>> = const { RefCell::new(None) };
}

/// `run_loading`'s app: kept for the loop that already runs (`None`), or given back to open one.
pub(super) fn hand_over(app: App) -> Option<App> {
    HANDED.with(|handed| match &mut *handed.borrow_mut() {
        Some(kept) => {
            *kept = Some(app);
            None
        }
        None => Some(app),
    })
}

struct Boot {
    job: Option<std::thread::JoinHandle<()>>,
    app: Option<App>,
    failed: bool,
}

/// Opens the event loop, runs `job` beside it and starts the game when `job` is done.
pub fn run_after(job: Box<dyn FnOnce() + Send>) -> super::Result<()> {
    let job = std::thread::Builder::new()
        .name("Unpacking resources".into())
        .spawn(job)?;
    let mut boot = Boot {
        job: Some(job),
        app: None,
        failed: false,
    };
    EventLoop::new()?.run_app(&mut boot)?;
    Ok(())
}

impl Boot {
    /// The game's own start, with the loop already running.
    fn start(&mut self, event_loop: &ActiveEventLoop) {
        HANDED.with(|handed| *handed.borrow_mut() = Some(None));
        let started = super::run_with();
        self.app = HANDED.with(|handed| handed.borrow_mut().take()).flatten();
        match (&mut self.app, started) {
            (Some(app), Ok(())) => {
                event_loop.set_control_flow(ControlFlow::Poll);
                // The loop's own `resumed` came before there was an app to hear it.
                app.resumed(event_loop);
            }
            (_, started) => {
                self.app = None;
                self.failed = true;
                let why = started.err().map_or_else(
                    || "the game did not start (are its resources in place?)".to_owned(),
                    |e| e.to_string(),
                );
                log::error!("Horizon: {why}");
                horizon_client::logging::flush();
                // The game folder (the app's Documents), where the Files app shows it.
                let _ = std::fs::write("ios-start-error.txt", format!("{why}\n"));
            }
        }
    }
}

impl ApplicationHandler for Boot {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(app) = &mut self.app {
            app.resumed(event_loop);
        }
    }
    fn suspended(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(app) = &mut self.app {
            app.suspended(event_loop);
        }
    }
    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        if let Some(app) = &mut self.app {
            app.window_event(event_loop, id, event);
        }
    }
    fn device_event(&mut self, event_loop: &ActiveEventLoop, id: DeviceId, event: DeviceEvent) {
        if let Some(app) = &mut self.app {
            app.device_event(event_loop, id, event);
        }
    }
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(app) = &mut self.app {
            return app.about_to_wait(event_loop);
        }
        if self.failed {
            event_loop.set_control_flow(ControlFlow::Wait);
            return;
        }
        if self.job.as_ref().is_some_and(|job| !job.is_finished()) {
            event_loop.set_control_flow(ControlFlow::WaitUntil(
                Instant::now() + Duration::from_millis(50),
            ));
            return;
        }
        if let Some(job) = self.job.take() {
            let _ = job.join();
        }
        self.start(event_loop);
    }
    fn exiting(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(app) = &mut self.app {
            app.exiting(event_loop);
        }
    }
}
