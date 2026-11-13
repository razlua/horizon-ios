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

/// The first start's unpacking: files done and in all (entry.rs), for the progress bar.
pub static UNPACKED: [std::sync::atomic::AtomicUsize; 2] =
    [const { std::sync::atomic::AtomicUsize::new(0) }; 2];

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
    /// The saved accounts, to play as one; `removing` while the buttons remove them.
    Choose { removing: bool },
    /// Signing a saved account in again.
    Resuming(String),
    /// Asking Microsoft for a code.
    Asking,
    /// Waiting for the player at Microsoft's page.
    Code { code: String, uri: String, link: String },
    /// Signed in: the session.
    Signed { name: String, session: String },
    /// It failed; the player retries, goes back or plays offline.
    Failed(String),
    /// Choosing the offline name: the one kept, or the one being typed.
    OfflineName {
        name: String,
        editing: bool,
        problem: Option<String>,
    },
    /// Offline, by the player's choice.
    Offline,
}

/// The on-device setup of the game files (ios_setup.rs), when the app carries none.
#[derive(Clone, Debug, PartialEq)]
enum Setup {
    /// The files are there (prepared, or unpacked from the app).
    NotNeeded,
    /// Waiting for a Microsoft sign-in: the files come with an account that owns the game.
    Waiting,
    Running,
    Failed(String),
    Done,
}

/// What the threads and the screen share.
struct Shared {
    unpacking: bool,
    setup: Setup,
    account: Account,
    /// The saved accounts, the one played last first.
    accounts: Vec<ios_account::Saved>,
    /// Counts the sign-ins started: one that is no longer the latest stops and changes nothing.
    attempt: u64,
    /// Counts up at each change, for the screen to draw again.
    version: u64,
    /// A newer build that is out, from `ios_update`.
    newer: Option<super::ios_update::Newer>,
}

type State = Arc<Mutex<Shared>>;

fn update(state: &State, f: impl FnOnce(&mut Shared)) {
    let mut shared = state.lock().unwrap_or_else(|e| e.into_inner());
    f(&mut shared);
    shared.version += 1;
}

/// Whether sign-in `attempt` is still the one the player wants.
fn current(state: &State, attempt: u64) -> bool {
    state.lock().unwrap_or_else(|e| e.into_inner()).attempt == attempt
}

/// Sets the account state, if `attempt` is still the latest sign-in.
fn settle(state: &State, attempt: u64, account: Account) {
    update(state, |s| {
        if s.attempt == attempt {
            s.account = account;
        }
    });
}

/// A new sign-in: the ones before it stop.
fn begin_attempt(state: &State, account: Account) -> u64 {
    let mut attempt = 0;
    update(state, |s| {
        s.attempt += 1;
        attempt = s.attempt;
        s.account = account;
    });
    attempt
}

fn signed_in(state: &State, attempt: u64, signed: ios_account::Signed) {
    if !current(state, attempt) {
        return;
    }
    ios_account::save(&signed);
    log::info!("Horizon iOS: signed in as {}", signed.name);
    let session = signed.session_json();
    let accounts = ios_account::saved();
    update(state, |s| {
        if s.attempt == attempt {
            s.accounts = accounts;
            s.account = Account::Signed {
                name: signed.name,
                session,
            };
        }
    });
}

/// Signs a saved account in again, on a thread of its own.
fn resume(state: State, attempt: u64, saved: ios_account::Saved) {
    match ios_account::resume(&saved) {
        Ok(signed) => signed_in(&state, attempt, signed),
        Err(e) => {
            log::warn!("Horizon iOS: the saved sign-in of {} did not work: {e}", saved.name);
            // A refused token is gone for good; no network keeps it for the next start.
            if !e.starts_with("no connection") {
                ios_account::forget(&saved);
                let accounts = ios_account::saved();
                update(&state, |s| s.accounts = accounts);
            }
            settle(&state, attempt, Account::Failed(e));
        }
    }
}

/// Signs a new account in with a device code, on a thread of its own.
fn sign_in_new(state: State, attempt: u64) {
    let code = match ios_account::begin() {
        Ok(code) => code,
        Err(e) => {
            log::warn!("Horizon iOS: {e}");
            return settle(&state, attempt, Account::Failed(e));
        }
    };
    settle(
        &state,
        attempt,
        Account::Code {
            code: code.user_code.clone(),
            uri: code.verification_uri.clone(),
            link: code.link(),
        },
    );
    let deadline = Instant::now() + code.expires_in;
    let mut interval = code.interval;
    let outcome = loop {
        std::thread::sleep(interval);
        if !current(&state, attempt) {
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
    match outcome {
        Ok(signed) => signed_in(&state, attempt, signed),
        Err(e) => {
            log::warn!("Horizon iOS: sign-in failed: {e}");
            settle(&state, attempt, Account::Failed(e));
        }
    }
}

/// Starts a thread for a sign-in.
fn spawn(name: &str, job: impl FnOnce() + Send + 'static) {
    if let Err(e) = std::thread::Builder::new().name(name.into()).spawn(job) {
        log::warn!("Horizon iOS: no {name} thread: {e}");
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
    /// The setup's downloads.
    progress: Arc<super::ios_setup::Progress>,
    /// The screen is kept on (while the setup downloads).
    awake: bool,
    /// The SwiftUI launcher is over this screen (`horizon_threads::ios_launcher`), which then
    /// isn't drawn.
    launcher: bool,
}

/// Opens the event loop, unpacks (`job`) or sets the game files up (`setup`) and signs in beside
/// it, and starts the game when both are done.
pub fn run_after(job: Option<Box<dyn FnOnce() + Send>>, setup: bool) -> super::Result<()> {
    let accounts = ios_account::saved();
    let first = accounts.is_empty();
    let state: State = Arc::new(Mutex::new(Shared {
        setup: if setup { Setup::Waiting } else { Setup::NotNeeded },
        unpacking: job.is_some(),
        account: Account::Choose { removing: false },
        accounts,
        attempt: 0,
        version: 1,
        newer: None,
    }));
    let checking = state.clone();
    spawn("Update check", move || {
        if let Some(newer) = super::ios_update::newer() {
            log::info!("Horizon iOS: version {} is out", newer.version);
            update(&checking, |s| s.newer = Some(newer));
        }
    });
    if let Some(job) = job {
        let unpacked = state.clone();
        std::thread::Builder::new()
            .name("Unpacking resources".into())
            .spawn(move || {
                job();
                update(&unpacked, |s| s.unpacking = false);
            })?;
    }
    // No saved account: straight to the sign-in.
    if first {
        let attempt = begin_attempt(&state, Account::Asking);
        let signing = state.clone();
        spawn("Sign-in", move || sign_in_new(signing, attempt));
    }
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
        progress: Arc::default(),
        awake: false,
        launcher: false,
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
        use horizon_client::ios_screen::Look;
        let shared = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let unpacking = shared
            .unpacking
            .then(|| "Unpacking your game files. This first start takes a minute.".to_owned());
        let [done, total] = UNPACKED
            .each_ref()
            .map(|v| v.load(std::sync::atomic::Ordering::Relaxed));
        let progress = (shared.unpacking && total > 0).then(|| done as f32 / total as f32);
        let mut screen = Screen {
            title: "Ferrite".into(),
            status: unpacking.clone(),
            progress,
            ..Default::default()
        };
        let back = !shared.accounts.is_empty();
        let button = |label: &str, action, look| (label.to_owned(), action, look);
        match &shared.account {
            Account::Choose { removing } => {
                screen.title = "Choose an account".into();
                if *removing {
                    screen.lines = vec!["Tap an account to remove it from this iPhone.".into()];
                    for (i, a) in shared.accounts.iter().enumerate() {
                        screen.buttons.push((
                            format!("Remove {}", a.name),
                            Action::Remove(i),
                            Look::Danger,
                        ));
                    }
                    screen.buttons.push(button("Done", Action::RemoveMode, Look::Plain));
                } else {
                    screen.lines =
                        vec!["Play as one of your Microsoft accounts, or add another.".into()];
                    for (i, a) in shared.accounts.iter().enumerate() {
                        let look = if i == 0 { Look::Primary } else { Look::Plain };
                        screen.buttons.push((a.name.clone(), Action::Play(i), look));
                    }
                    screen.buttons.push(button("Add account", Action::AddAccount, Look::Plain));
                    screen
                        .buttons
                        .push(button("Remove an account", Action::RemoveMode, Look::Plain));
                    screen.buttons.push(button("Play offline", Action::Offline, Look::Plain));
                }
                if let Some(newer) = &shared.newer {
                    let installed = horizon_threads::ios_bundle::info("CFBundleShortVersionString")
                        .unwrap_or_else(|| "an older one".into());
                    let version = &newer.version;
                    screen.lines.push(String::new());
                    // With SideStore or AltStore, one tap installs it; else the download page.
                    let (line, label) = match super::ios_update::installer() {
                        Some(name) => (
                            format!(
                                "Update available: version {version} is out (you have \
                                 {installed}). {name} installs it over this app; your worlds \
                                 and sign-ins stay."
                            ),
                            format!("Update with {name}"),
                        ),
                        None => (
                            format!(
                                "Update available: version {version} is out (you have \
                                 {installed}). Download it and sideload it over this app; your \
                                 worlds and sign-ins stay."
                            ),
                            "Get the update".to_owned(),
                        ),
                    };
                    screen.lines.push(line);
                    screen.buttons.push((label, Action::GetUpdate, Look::Primary));
                }
            }
            Account::Resuming(name) => {
                screen.lines = vec![format!("Signing in as {name}...")];
                screen.buttons.push(button("Back", Action::Back, Look::Plain));
            }
            Account::Asking => {
                screen.lines = vec!["Connecting to Microsoft...".into()];
                if back {
                    screen.buttons.push(button("Back", Action::Back, Look::Plain));
                }
                screen.buttons.push(button("Play offline", Action::Offline, Look::Plain));
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
                screen
                    .buttons
                    .push(button("Open sign-in page", Action::OpenLink, Look::Primary));
                if back {
                    screen.buttons.push(button("Back", Action::Back, Look::Plain));
                }
                screen.buttons.push(button("Play offline", Action::Offline, Look::Plain));
                screen.status = unpacking.or(Some("Waiting for you to sign in...".into()));
            }
            Account::Signed { name, .. } => {
                screen.lines = vec![format!("Signed in as {name}.")];
                screen.status = unpacking.or(Some("Starting...".into()));
            }
            Account::Failed(why) => {
                screen.title = "Sign-in did not work".into();
                screen.lines = vec![why.clone()];
                screen.buttons.push(button("Try again", Action::Retry, Look::Primary));
                if back {
                    screen.buttons.push(button("Back", Action::Back, Look::Plain));
                }
                screen.buttons.push(button("Play offline", Action::Offline, Look::Plain));
            }
            Account::OfflineName {
                name,
                editing,
                problem,
            } => {
                screen.title = "Play offline".into();
                screen.lines = vec![
                    "Your name in single player and on offline-mode (cracked) servers: 3 to 16 \
                     letters, numbers or _."
                        .into(),
                ];
                screen.code = Some(if *editing {
                    format!("{name}_")
                } else {
                    name.clone()
                });
                if *editing {
                    screen.buttons.push(button("Done", Action::ChangeName, Look::Primary));
                } else {
                    screen.buttons.push(button("Play", Action::PlayOffline, Look::Primary));
                    screen.buttons.push(button("Change name", Action::ChangeName, Look::Plain));
                    screen.buttons.push(button("Back", Action::Back, Look::Plain));
                }
                screen.status = problem.clone().or(unpacking);
            }
            Account::Offline => {
                screen.lines = vec!["Playing offline.".into()];
                screen.status = unpacking.or(Some("Starting...".into()));
            }
        }
        // The game files' setup takes the screen once it runs, or when it can't yet.
        use std::sync::atomic::Ordering::Relaxed;
        match (&shared.setup, &shared.account) {
            (Setup::Running | Setup::Waiting, Account::Signed { .. }) => {
                let (done, total) = (self.progress.done.load(Relaxed), self.progress.total.load(Relaxed));
                let stage = self.progress.stage.lock().unwrap_or_else(|e| e.into_inner()).clone();
                screen.title = "Getting the game files".into();
                screen.lines = vec![
                    "Ferrite downloads your Minecraft 26.3 from Mojang (about 520 MB) and its own \
                     tables, and sets them up. This happens once. Keep Ferrite open, on Wi-Fi."
                        .into(),
                ];
                screen.code = None;
                screen.buttons.clear();
                screen.status = Some(if total > 0 {
                    format!("{stage}: {} of {} MB", done / 1_000_000, total / 1_000_000)
                } else {
                    format!("{stage}...")
                });
                screen.progress = Some(if total > 0 { done as f32 / total as f32 } else { 0.0 });
            }
            (Setup::Failed(why), _) => {
                screen.title = "The download stopped".into();
                screen.lines = vec![why.clone(), "Try again goes on where it stopped.".into()];
                screen.code = None;
                screen.buttons = vec![button("Try again", Action::RetrySetup, Look::Primary)];
                screen.status = None;
                screen.progress = None;
            }
            (Setup::Waiting, Account::Offline) => {
                screen.title = "Sign in once first".into();
                screen.lines = vec![
                    "Ferrite downloads the game files from Mojang with a Microsoft account that \
                     owns Minecraft: Java Edition. Sign in once; after that you can play offline \
                     too."
                        .into(),
                ];
                screen.buttons = vec![button("Sign in", Action::Back, Look::Primary)];
                screen.status = None;
            }
            _ => {}
        }
        screen
    }

    /// A key while the offline name is being typed: letters, digits and _ go in, Backspace
    /// takes one out, return finishes.
    fn type_name(&mut self, key: &winit::keyboard::Key) {
        use winit::keyboard::{Key, NamedKey};
        let mut finished = false;
        update(&self.state, |s| {
            let Account::OfflineName {
                name,
                editing: editing @ true,
                problem,
            } = &mut s.account
            else {
                return;
            };
            *problem = None;
            match key {
                Key::Named(NamedKey::Backspace) => {
                    name.pop();
                }
                Key::Named(NamedKey::Enter) => {
                    *editing = false;
                    finished = true;
                }
                Key::Character(text) if text == "\n" || text == "\r" => {
                    *editing = false;
                    finished = true;
                }
                Key::Character(text) => {
                    for c in text.chars() {
                        if (c.is_ascii_alphanumeric() || c == '_') && name.len() < 16 {
                            name.push(c);
                        }
                    }
                }
                _ => {}
            }
        });
        if finished && let Some(window) = &self.window {
            window.set_ime_allowed(false);
        }
        self.drawn = 0;
    }

    /// Starts the game files' setup (ios_setup.rs) on a thread of its own.
    fn start_setup(&self) {
        use std::sync::atomic::Ordering::Relaxed;
        update(&self.state, |s| s.setup = Setup::Running);
        self.progress.done.store(0, Relaxed);
        self.progress.total.store(0, Relaxed);
        self.progress.cancel.store(false, Relaxed);
        let (state, progress) = (self.state.clone(), self.progress.clone());
        spawn("Game files", move || {
            let resources = std::env::current_dir()
                .map(|d| d.join("resources"))
                .unwrap_or_else(|_| "resources".into());
            let work = std::env::var_os("HOME")
                .map(std::path::PathBuf::from)
                .unwrap_or_default()
                .join("Library/Caches/horizon-setup");
            let result = super::ios_setup::run(&resources, &work, &progress);
            if let Err(e) = &result {
                log::warn!("Horizon setup: {e}");
            }
            update(&state, |s| {
                s.setup = match result {
                    Ok(()) => Setup::Done,
                    Err(e) => Setup::Failed(e),
                }
            });
        });
    }

    fn with_ui<R>(
        &self,
        batch: &mut HudBatch,
        f: impl FnOnce(&mut horizon_client::ui_theme::Ui) -> R,
    ) -> Option<R> {
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
        let state = self.state.clone();
        let account = |i: usize| {
            state
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .accounts
                .get(i)
                .cloned()
        };
        match action {
            Action::OpenLink => {
                let link = match &state.lock().unwrap_or_else(|e| e.into_inner()).account {
                    Account::Code { link, .. } => Some(link.clone()),
                    _ => None,
                };
                if let Some(link) = link
                    && !horizon_threads::ios_url::open_url(&link)
                {
                    log::warn!("Horizon iOS: could not open the sign-in page");
                }
            }
            Action::Retry | Action::AddAccount => {
                let attempt = begin_attempt(&state, Account::Asking);
                spawn("Sign-in", move || sign_in_new(state, attempt));
            }
            Action::Play(i) => {
                if let Some(saved) = account(i) {
                    let attempt = begin_attempt(&state, Account::Resuming(saved.name.clone()));
                    spawn("Sign-in", move || resume(state, attempt, saved));
                }
            }
            Action::RemoveMode => update(&state, |s| {
                if let Account::Choose { removing } = &mut s.account {
                    *removing = !*removing;
                }
            }),
            Action::Remove(i) => {
                if let Some(saved) = account(i) {
                    ios_account::forget(&saved);
                    let accounts = ios_account::saved();
                    let none_left = accounts.is_empty();
                    update(&state, |s| s.accounts = accounts);
                    if none_left {
                        let attempt = begin_attempt(&state, Account::Asking);
                        spawn("Sign-in", move || sign_in_new(state, attempt));
                    }
                }
            }
            Action::Back => {
                let accounts = ios_account::saved();
                if accounts.is_empty() {
                    // Nothing to go back to: a new sign-in.
                    let attempt = begin_attempt(&state, Account::Asking);
                    spawn("Sign-in", move || sign_in_new(state, attempt));
                } else {
                    begin_attempt(&state, Account::Choose { removing: false });
                    update(&state, |s| s.accounts = accounts);
                }
            }
            Action::RetrySetup => self.start_setup(),
            // The name to play offline with first (horizon-player.txt, as the game keeps it).
            Action::Offline => {
                let name = horizon_client::offline_name::load_or_create(
                    &horizon_client::logging::player_dir(),
                );
                begin_attempt(
                    &state,
                    Account::OfflineName {
                        name,
                        editing: false,
                        problem: None,
                    },
                );
            }
            Action::ChangeName => {
                let mut editing_now = false;
                update(&state, |s| {
                    if let Account::OfflineName { editing, problem, .. } = &mut s.account {
                        *editing = !*editing;
                        *problem = None;
                        editing_now = *editing;
                    }
                });
                if let Some(window) = &self.window {
                    window.set_ime_allowed(editing_now);
                }
            }
            Action::GetUpdate => {
                let newer = state.lock().unwrap_or_else(|e| e.into_inner()).newer.clone();
                let link = newer.as_ref().map_or_else(
                    || super::ios_update::RELEASES.to_owned(),
                    super::ios_update::update_link,
                );
                if !horizon_threads::ios_url::open_url(&link) {
                    log::warn!("Horizon iOS: could not open {link}");
                }
            }
            Action::PlayOffline => {
                let chosen = match &state.lock().unwrap_or_else(|e| e.into_inner()).account {
                    Account::OfflineName { name, .. } => Some(name.clone()),
                    _ => None,
                };
                if let Some(name) = chosen {
                    if horizon_client::offline_name::is_chosen_valid(&name) {
                        let dir = horizon_client::logging::player_dir();
                        if let Err(e) = horizon_client::offline_name::save(&dir, &name) {
                            log::warn!("Horizon iOS: the offline name could not be kept: {e}");
                        }
                        log::info!("Horizon iOS: playing offline as {name}");
                        begin_attempt(&state, Account::Offline);
                    } else {
                        update(&state, |s| {
                            if let Account::OfflineName { problem, .. } = &mut s.account {
                                *problem = Some("Use 3 to 16 letters, numbers or _.".into());
                            }
                        });
                    }
                }
            }
        }
        let _ = event_loop;
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    /// The game can start: unpacked, and signed in or offline. Its session, if any.
    fn ready(&self) -> Option<Option<String>> {
        let shared = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if shared.unpacking || !matches!(shared.setup, Setup::NotNeeded | Setup::Done) {
            return None;
        }
        match &shared.account {
            Account::Signed { session, .. } => Some(Some(session.clone())),
            Account::Offline => Some(None),
            _ => None,
        }
    }

    /// The game's own start, with the loop already running.
    /// The SwiftUI launcher over this screen, where the build has it, with what it shows.
    fn show_launcher(&mut self) {
        use horizon_threads::ios_launcher;
        if !ios_launcher::available() || self.launcher {
            return;
        }
        self.launcher = ios_launcher::show();
        if self.launcher {
            let version = horizon_threads::ios_bundle::info("CFBundleShortVersionString");
            ios_launcher::update(&serde_json::json!({ "version": version }).to_string());
        }
        log::info!("Horizon iOS: SwiftUI launcher shown: {}", self.launcher);
    }

    /// What the player tapped in the launcher. "continue": this screen, as before.
    fn launcher_actions(&mut self) {
        for action in horizon_threads::ios_launcher::take_actions() {
            match action.as_str() {
                "continue" => self.hide_launcher(),
                other => log::warn!("Horizon iOS: the launcher sent an unknown action {other:?}"),
            }
        }
    }

    fn hide_launcher(&mut self) {
        if self.launcher {
            horizon_threads::ios_launcher::hide();
            self.launcher = false;
            // This screen draws again.
            self.drawn = 0;
        }
    }

    fn start(&mut self, event_loop: &ActiveEventLoop, session: Option<String>) {
        // The launcher goes before the game starts.
        self.hide_launcher();
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
        horizon_threads::ios_keyboard::watch();
        if let Some(rate) = horizon_threads::ios_display::high_frame_rate() {
            log::info!("Horizon iOS: asked for {rate} frames per second");
        }
        if let Some(app) = &mut self.app {
            return app.resumed(event_loop);
        }
        if self.window.is_some() || self.failed {
            return;
        }
        let window = match event_loop
            .create_window(phone_window(Window::default_attributes().with_title("Ferrite")))
        {
            Ok(window) => Arc::new(window),
            // The unpacking and the sign-in go on without a screen.
            Err(e) => return log::warn!("Horizon iOS: no start screen: {e}"),
        };
        self.window = Some(window.clone());
        // The launcher first: it needs only the window.
        self.show_launcher();
        // The iOS Simulator's GPU can't run the renderer (no indirect drawing): there the
        // launcher alone shows, for the screenshots of the simulator job (ios.yml).
        if cfg!(target_abi = "sim") {
            return;
        }
        let made = (|| -> super::Result<_> {
            let gui = UiResources {
                atlas: HudAtlas::build(std::path::Path::new("resources"))?,
                panorama: Arc::new(horizon_client::gfx::panorama::Faces::colour([0; 4])),
            };
            let renderer = loading_renderer(
                window.clone(),
                false,
                Some(&gui.atlas),
                Some(&gui.panorama),
                None,
                horizon_client::graphics::GraphicsApi::Auto,
            )?;
            Ok((gui, renderer))
        })();
        match made {
            Ok((gui, renderer)) => {
                window.request_redraw();
                self.gui = Some(gui);
                self.renderer = Some(renderer);
            }
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
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                self.type_name(&event.logical_key);
            }
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
        // Signed in with the files still to come: the setup starts.
        let (waiting, running) = {
            let shared = self.state.lock().unwrap_or_else(|e| e.into_inner());
            (
                shared.setup == Setup::Waiting && matches!(shared.account, Account::Signed { .. }),
                shared.setup == Setup::Running,
            )
        };
        if waiting {
            self.start_setup();
        }
        if running != self.awake {
            self.awake = running;
            horizon_threads::ios_display::keep_awake(running);
        }
        self.launcher_actions();
        if let Some(session) = self.ready() {
            return self.start(event_loop, session);
        }
        // The unpacking's and the setup's progress draw again too.
        let version = self.state.lock().unwrap_or_else(|e| e.into_inner()).version
            + UNPACKED[0].load(std::sync::atomic::Ordering::Relaxed) as u64
            + self.progress.done.load(std::sync::atomic::Ordering::Relaxed) / 1_000_000;
        if version != self.drawn && !self.hidden && !self.launcher {
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
