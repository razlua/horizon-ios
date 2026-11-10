//! iOS: what runs before the game. The system ends an app that takes too long to start, and the
//! first start unpacks the resources (entry.rs) for longer than that; and with no launcher, the
//! game signs the player in itself (ios_account.rs). So the event loop starts at once with a
//! screen of its own (`horizon_client::ios_screen`), the unpacking and the sign-in run on threads
//! beside it, and the game starts inside the running loop when both are done: `run_loading`
//! hands its app over instead of opening a loop.

use super::ios_account::{self, Poll};
use super::*;
use horizon_client::ios_screen::{Action, Screen};
use std::cell::RefCell;
use std::sync::Mutex;
use winit::event::TouchPhase;

thread_local! {
    /// `Some` while the loop of `run_after` waits for the game's app, then the app itself.
    static HANDED: RefCell<Option<Option<App>>> = const { RefCell::new(None) };
}

/// The signed-in session for `Options::read`, taken once.
static SESSION: Mutex<Option<String>> = Mutex::new(None);

/// The session the sign-in made, if any (`Options::read`).
pub(super) fn take_session() -> Option<String> {
    SESSION.lock().unwrap_or_else(|e| e.into_inner()).take()
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

/// Where the sign-in stands.
#[derive(Clone, Debug, PartialEq)]
enum Account {
    /// Signing the kept account in again.
    Resuming(String),
    /// Asking Microsoft for a code.
    Asking,
    /// Waiting for the player at Microsoft's page.
    Code { code: String, uri: String, link: String },
    /// Signed in: the session.
    Signed { name: String, session: String },
    /// It failed; the player retries or plays offline.
    Failed(String),
    /// Offline, by the player's choice.
    Offline,
}

/// What the threads and the screen share.
struct Shared {
    unpacking: bool,
    account: Account,
    /// The player chose: the sign-in thread stops looking.
    cancelled: bool,
    /// Counts up at each change, for the screen to draw again.
    version: u64,
}

type State = Arc<Mutex<Shared>>;

fn update(state: &State, f: impl FnOnce(&mut Shared)) {
    let mut shared = state.lock().unwrap_or_else(|e| e.into_inner());
    f(&mut shared);
    shared.version += 1;
}

fn cancelled(state: &State) -> bool {
    state.lock().unwrap_or_else(|e| e.into_inner()).cancelled
}

/// The sign-in, on a thread of its own: the kept account, else a device code.
fn sign_in(state: State, use_saved: bool) {
    let saved = if use_saved { ios_account::saved() } else { None };
    if let Some(saved) = saved {
        update(&state, |s| s.account = Account::Resuming(saved.name.clone()));
        match ios_account::resume(&saved) {
            Ok(signed) => {
                ios_account::save(&signed);
                log::info!("Horizon iOS: signed in again as {}", signed.name);
                let session = signed.session_json();
                update(&state, |s| {
                    s.account = Account::Signed {
                        name: signed.name,
                        session,
                    }
                });
                return;
            }
            Err(e) => {
                log::warn!("Horizon iOS: the saved sign-in did not work: {e}");
                // A refused token is gone for good; no network keeps it for the next start.
                if !e.starts_with("no connection") {
                    ios_account::forget();
                }
                update(&state, |s| s.account = Account::Failed(e));
                return;
            }
        }
    }
    update(&state, |s| s.account = Account::Asking);
    let code = match ios_account::begin() {
        Ok(code) => code,
        Err(e) => {
            log::warn!("Horizon iOS: {e}");
            update(&state, |s| s.account = Account::Failed(e));
            return;
        }
    };
    update(&state, |s| {
        s.account = Account::Code {
            code: code.user_code.clone(),
            uri: code.verification_uri.clone(),
            link: code.link(),
        }
    });
    let deadline = Instant::now() + code.expires_in;
    let mut interval = code.interval;
    let outcome = loop {
        std::thread::sleep(interval);
        if cancelled(&state) {
            return;
        }
        if Instant::now() > deadline {
            break Err("The code expired.".to_owned());
        }
        match ios_account::poll(&code) {
            Ok(Poll::Pending) => {}
            Ok(Poll::SlowDown) => interval += Duration::from_secs(5),
            Ok(Poll::Done(access, refresh)) => break ios_account::minecraft(&access, refresh),
            // The app was in the background (Safari): the connection can drop. Look again.
            Err(e) if e.starts_with("no connection") => {}
            Err(e) => break Err(e),
        }
    };
    if cancelled(&state) {
        return;
    }
    match outcome {
        Ok(signed) => {
            ios_account::save(&signed);
            log::info!("Horizon iOS: signed in as {}", signed.name);
            let session = signed.session_json();
            update(&state, |s| {
                s.account = Account::Signed {
                    name: signed.name,
                    session,
                }
            });
        }
        Err(e) => {
            log::warn!("Horizon iOS: sign-in failed: {e}");
            update(&state, |s| s.account = Account::Failed(e));
        }
    }
}

struct Boot {
    state: State,
    drawn: u64,
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    gui: Option<UiResources>,
    /// The pointer in physical pixels.
    pointer: (f64, f64),
    hidden: bool,
    app: Option<App>,
    failed: bool,
}

/// Opens the event loop, unpacks (`job`) and signs in beside it, and starts the game when both
/// are done.
pub fn run_after(job: Option<Box<dyn FnOnce() + Send>>) -> super::Result<()> {
    let state: State = Arc::new(Mutex::new(Shared {
        unpacking: job.is_some(),
        account: Account::Asking,
        cancelled: false,
        version: 1,
    }));
    if let Some(job) = job {
        let unpacked = state.clone();
        std::thread::Builder::new()
            .name("Unpacking resources".into())
            .spawn(move || {
                job();
                update(&unpacked, |s| s.unpacking = false);
            })?;
    }
    let signing = state.clone();
    std::thread::Builder::new()
        .name("Sign-in".into())
        .spawn(move || sign_in(signing, true))?;
    let mut boot = Boot {
        state,
        drawn: 0,
        window: None,
        renderer: None,
        gui: None,
        pointer: (0.0, 0.0),
        hidden: false,
        app: None,
        failed: false,
    };
    EventLoop::new()?.run_app(&mut boot)?;
    Ok(())
}

impl Boot {
    fn size(&self) -> [u32; 2] {
        self.renderer
            .as_ref()
            .map_or([1280, 720], |r| [r.config.width, r.config.height])
    }

    fn screen(&self) -> Screen {
        let shared = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let unpacking = shared.unpacking.then(|| {
            "Unpacking your game files. This first start takes a minute.".to_owned()
        });
        let mut screen = Screen {
            title: "Horizon".into(),
            status: unpacking.clone(),
            ..Default::default()
        };
        match &shared.account {
            Account::Resuming(name) => {
                screen.lines = vec![format!("Signing in as {name}...")];
            }
            Account::Asking => {
                screen.lines = vec!["Connecting to Microsoft...".into()];
                screen.buttons = vec![("Play offline", Action::Offline)];
            }
            Account::Code { code, uri, .. } => {
                screen.title = "Sign in with Microsoft".into();
                screen.lines = vec![
                    "Tap Open sign-in page and sign in with the Microsoft account that owns \
                     Minecraft: Java Edition, then come back here."
                        .into(),
                    format!("Or open {uri} on any device and enter this code:"),
                ];
                screen.code = Some(code.clone());
                screen.buttons = vec![
                    ("Open sign-in page", Action::OpenLink),
                    ("Play offline", Action::Offline),
                ];
                screen.status = unpacking.or(Some("Waiting for you to sign in...".into()));
            }
            Account::Signed { name, .. } => {
                screen.lines = vec![format!("Signed in as {name}.")];
                screen.status = unpacking.or(Some("Starting...".into()));
            }
            Account::Failed(why) => {
                screen.title = "Sign-in did not work".into();
                screen.lines = vec![why.clone()];
                screen.buttons = vec![
                    ("Try again", Action::Retry),
                    ("Play offline", Action::Offline),
                ];
            }
            Account::Offline => {
                screen.lines = vec!["Playing offline.".into()];
                screen.status = unpacking.or(Some("Starting...".into()));
            }
        }
        screen
    }

    fn with_ui<R>(&self, batch: &mut HudBatch, f: impl FnOnce(&mut horizon_client::ui_theme::Ui) -> R) -> Option<R> {
        let gui = self.gui.as_ref()?;
        let cursor = (
            self.pointer.0 as f32 / batch.scale,
            self.pointer.1 as f32 / batch.scale,
        );
        let mut ui = horizon_client::ui_theme::Ui::new(&gui.atlas, batch, cursor);
        let result = f(&mut ui);
        ui.finish();
        Some(result)
    }

    fn draw(&mut self) {
        if self.hidden {
            return;
        }
        let screen = self.screen();
        let mut batch = HudBatch::new(self.size());
        if self.with_ui(&mut batch, |ui| screen.draw(ui)).is_none() {
            return;
        }
        let Some(renderer) = &mut self.renderer else {
            return;
        };
        match renderer.render(&loading_frame(&batch)) {
            Ok(true) => return,
            Ok(false) => {}
            Err(e) => log::warn!("Horizon: frame failed: {e}"),
        }
        // Not shown: the glyphs it rasterized never reached the atlas, so draw again.
        horizon_client::ui_theme::reset_atlas();
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    fn click(&mut self, event_loop: &ActiveEventLoop) {
        let screen = self.screen();
        let mut batch = HudBatch::new(self.size());
        let Some(Some(action)) = self.with_ui(&mut batch, |ui| screen.click(ui)) else {
            return;
        };
        match action {
            Action::OpenLink => {
                let link = match &self.state.lock().unwrap_or_else(|e| e.into_inner()).account {
                    Account::Code { link, .. } => Some(link.clone()),
                    _ => None,
                };
                if let Some(link) = link
                    && !horizon_threads::ios_url::open_url(&link)
                {
                    log::warn!("Horizon iOS: could not open the sign-in page");
                }
            }
            Action::Retry => {
                update(&self.state, |s| {
                    s.cancelled = false;
                    s.account = Account::Asking;
                });
                let state = self.state.clone();
                let _ = std::thread::Builder::new()
                    .name("Sign-in".into())
                    .spawn(move || sign_in(state, false));
            }
            Action::Offline => update(&self.state, |s| {
                s.cancelled = true;
                s.account = Account::Offline;
            }),
        }
        let _ = event_loop;
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    /// The game can start: unpacked, and signed in or offline. Its session, if any.
    fn ready(&self) -> Option<Option<String>> {
        let shared = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if shared.unpacking {
            return None;
        }
        match &shared.account {
            Account::Signed { session, .. } => Some(Some(session.clone())),
            Account::Offline => Some(None),
            _ => None,
        }
    }

    /// The game's own start, with the loop already running.
    fn start(&mut self, event_loop: &ActiveEventLoop, session: Option<String>) {
        // The game opens its own window: this one goes first.
        self.renderer = None;
        self.window = None;
        *SESSION.lock().unwrap_or_else(|e| e.into_inner()) = session;
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
        self.hidden = false;
        if let Some(rate) = horizon_threads::ios_display::high_frame_rate() {
            log::info!("Horizon iOS: asked for {rate} frames per second");
        }
        if let Some(app) = &mut self.app {
            return app.resumed(event_loop);
        }
        if self.window.is_some() || self.failed {
            return;
        }
        let made = (|| -> super::Result<_> {
            let gui = UiResources {
                atlas: HudAtlas::build(std::path::Path::new("resources"))?,
                panorama: Arc::new(horizon_client::gfx::panorama::Faces::colour([0; 4])),
            };
            let window = Arc::new(
                event_loop.create_window(phone_window(Window::default_attributes().with_title("Horizon")))?,
            );
            let renderer = loading_renderer(
                window.clone(),
                false,
                Some(&gui.atlas),
                Some(&gui.panorama),
                None,
                horizon_client::graphics::GraphicsApi::Auto,
            )?;
            Ok((gui, window, renderer))
        })();
        match made {
            Ok((gui, window, renderer)) => {
                window.request_redraw();
                self.gui = Some(gui);
                self.window = Some(window);
                self.renderer = Some(renderer);
            }
            // The unpacking and the sign-in go on without a screen.
            Err(e) => log::warn!("Horizon iOS: no start screen: {e}"),
        }
    }
    fn suspended(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(app) = &mut self.app {
            app.suspended(event_loop);
        }
    }
    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        if let Some(app) = &mut self.app {
            return app.window_event(event_loop, id, event);
        }
        if self.window.as_ref().is_none_or(|w| w.id() != id) {
            return;
        }
        match event {
            WindowEvent::Occluded(hidden) => self.hidden = hidden,
            WindowEvent::Resized(size) => {
                if let Some(r) = &mut self.renderer {
                    r.resize(size.width, size.height);
                }
                self.drawn = 0;
            }
            WindowEvent::RedrawRequested => self.draw(),
            WindowEvent::CursorMoved { position, .. } => {
                self.pointer = (position.x, position.y);
                self.drawn = 0;
            }
            WindowEvent::MouseInput {
                state: ElementState::Released,
                button: MouseButton::Left,
                ..
            } => self.click(event_loop),
            WindowEvent::Touch(touch) => {
                let at = match &self.window {
                    Some(window) => super::touch::touch_pixels(window, self.size(), &touch),
                    None => [touch.location.x, touch.location.y],
                };
                self.pointer = (at[0], at[1]);
                self.drawn = 0;
                if touch.phase == TouchPhase::Ended {
                    self.click(event_loop);
                }
            }
            _ => {}
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
        if let Some(session) = self.ready() {
            return self.start(event_loop, session);
        }
        let version = self.state.lock().unwrap_or_else(|e| e.into_inner()).version;
        if version != self.drawn && !self.hidden {
            self.drawn = version;
            if let Some(window) = &self.window {
                window.request_redraw();
            }
        }
        event_loop.set_control_flow(ControlFlow::WaitUntil(
            Instant::now() + Duration::from_millis(100),
        ));
    }
    fn exiting(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(app) = &mut self.app {
            app.exiting(event_loop);
        }
    }
}
