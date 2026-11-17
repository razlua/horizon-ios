//! Ferrite (iOS): signing Ferrite again with the player's Apple ID, so it keeps opening after its
//! free 7-day signature runs out. The launcher's Refresh tab (packaging/ios/swift) drives it.
//!
//! One worker thread owns the Apple session (isideload) and runs the commands sent to it: sign
//! in (with 2FA), check (what a new signature would be), sign out. The launcher reads
//! [`state_json`] whenever [`version`] changes.
//!
//! The Apple ID's password reaches this crate only through [`hz_signer_sign_in`], called by
//! Swift with the text of its secure field. It is passed to Apple's sign-in (SRP: the password
//! itself never leaves the phone) and dropped: never stored, never logged, never in the state.
//!
//! Safety rules kept for the player's other sideloaded apps: no certificate is ever revoked
//! (`MaxCertsBehavior::Error`), and Ferrite is signed under the exact bundle ID it is installed
//! with, so iOS updates the same app instead of installing a second one without its worlds.
//!
//! Refresh now: with LocalDevVPN on, the iPhone's own installer answers at 10.7.0.1, and the
//! player's pairing file (made once on a computer) lets Ferrite in. Ferrite registers the iPhone
//! with the Apple ID, gets a new 7-day profile (asking for the increased memory limit too),
//! copies its own app, signs the copy and installs it over itself. iOS closes Ferrite while it
//! installs; it opens again with 7 new days, its worlds and sign-ins kept.
#![cfg(target_os = "ios")]

use std::ffi::{CStr, c_char};
use std::net::{IpAddr, Ipv4Addr};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, SystemTime};

use idevice::pairing_file::PairingFile;
use idevice::provider::TcpProvider;
use isideload::auth::apple_account::{
    AppleAccount, TwoFactorCallbackParams, TwoFactorCallbackResponse,
};
use isideload::dev::app_ids::{AppId, AppIdsApi};
use isideload::dev::developer_session::DeveloperSession;
use isideload::dev::devices::DevicesApi;
use isideload::dev::teams::{DeveloperTeam, TeamsApi};
use isideload::sideload::application::{Application, SpecialApp};
use isideload::sideload::builder::MaxCertsBehavior;
use isideload::sideload::cert_identity::CertificateIdentity;
use isideload::util::device::IdeviceInfo;
use isideload::util::storage::new_storage;

/// The name Ferrite's signing certificate carries in the Apple ID's developer account.
const MACHINE_NAME: &str = "Ferrite";
/// How long the 2FA code is waited for.
const CODE_WAIT: Duration = Duration::from_secs(300);
/// Where LocalDevVPN puts the iPhone itself.
const DEVICE_ADDR: IpAddr = IpAddr::V4(Ipv4Addr::new(10, 7, 0, 1));
/// How long the iPhone's installer gets to answer.
const CONNECT_WAIT: Duration = Duration::from_secs(10);
/// What Test connection and Refresh now say without a pairing file.
const NO_PAIRING: &str = "No pairing file yet. Put it in Files, On My iPhone, Ferrite, pairing.";

/// Where the signing stands, as the Refresh tab shows it.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum Status {
    #[default]
    SignedOut,
    SigningIn,
    /// Apple sent a code to the player's devices (or by SMS): it's typed in the launcher. With
    /// what went wrong with the last one, and the phone number an SMS would go to.
    NeedsCode { last_error: Option<String>, sms_to: Option<String> },
    /// Busy, with what it's doing.
    Working(String),
    SignedIn,
    Failed(String),
}

#[derive(Default)]
struct State {
    status: Status,
    email: Option<String>,
    team: Option<String>,
    /// The last check: the bundle ID and when a new signature would run out (Unix seconds).
    checked: Option<(String, u64)>,
    /// How far a refresh is, 0 to 1, while it runs.
    progress: Option<f32>,
    /// The iPhone's name, once Ferrite reached its installer.
    device: Option<String>,
    /// How the last Test connection or Refresh now ended: worked, and what to say.
    refresh_note: Option<(bool, String)>,
}

static STATE: Mutex<Option<State>> = Mutex::new(None);
static VERSION: AtomicU64 = AtomicU64::new(1);

enum Command {
    SignIn { email: String, password: String },
    Check { bundle_id: String },
    Test { pairing: Option<PathBuf> },
    Refresh { bundle_id: String, app: PathBuf, work: PathBuf, pairing: Option<PathBuf> },
    Update { url: String, bundle_id: String, work: PathBuf, pairing: Option<PathBuf> },
    SignOut,
}

struct Channels {
    commands: Mutex<Sender<Command>>,
    codes: Mutex<tokio::sync::mpsc::UnboundedSender<String>>,
}

static CHANNELS: OnceLock<Option<Channels>> = OnceLock::new();

fn change(f: impl FnOnce(&mut State)) {
    let mut guard = STATE.lock().unwrap_or_else(|e| e.into_inner());
    f(guard.get_or_insert_with(State::default));
    VERSION.fetch_add(1, Relaxed);
}

fn set_status(status: Status) {
    change(|s| s.status = status);
}

/// Counts up at each change of [`state_json`].
pub fn version() -> u64 {
    VERSION.load(Relaxed)
}

/// What the Refresh tab shows (packaging/ios/swift/Model.swift, `SignerState`).
pub fn state_json() -> serde_json::Value {
    let guard = STATE.lock().unwrap_or_else(|e| e.into_inner());
    let empty = State::default();
    let state = guard.as_ref().unwrap_or(&empty);
    let (status, message) = match &state.status {
        Status::SignedOut => ("signed_out", None),
        Status::SigningIn => ("signing_in", None),
        Status::NeedsCode { last_error, .. } => ("needs_code", last_error.clone()),
        Status::Working(what) => ("working", Some(what.clone())),
        Status::SignedIn => ("signed_in", None),
        Status::Failed(why) => ("failed", Some(why.clone())),
    };
    serde_json::json!({
        "status": status,
        "message": message,
        "email": state.email,
        "team": state.team,
        "sms_to": match &state.status {
            Status::NeedsCode { sms_to, .. } => sms_to.clone(),
            _ => None,
        },
        "checked_bundle_id": state.checked.as_ref().map(|c| c.0.clone()),
        "checked_expires": state.checked.as_ref().map(|c| c.1),
        "progress": state.progress,
        "device": state.device,
        "refresh_ok": state.refresh_note.as_ref().map(|n| n.0),
        "refresh_note": state.refresh_note.as_ref().map(|n| n.1.clone()),
    })
}

/// A Test connection or Refresh now ended: the Apple ID stays as it was, the note says how.
fn finish(signed_in: bool, ok: bool, note: String) {
    change(|s| {
        s.progress = None;
        s.refresh_note = Some((ok, note));
        s.status = if signed_in { Status::SignedIn } else { Status::SignedOut };
    });
}

/// A refresh's step: what it's doing, and how far it is.
fn step(what: &str, progress: f32) {
    log::info!("Ferrite signer: {what}");
    change(|s| {
        s.status = Status::Working(what.to_owned());
        s.progress = Some(progress);
    });
}

/// The worker thread and its channels, started the first time they're needed.
fn channels() -> Option<&'static Channels> {
    CHANNELS
        .get_or_init(|| {
            let (commands, received) = channel();
            let (codes, code_rx) = tokio::sync::mpsc::unbounded_channel();
            let started = std::thread::Builder::new()
                .name("Signer".into())
                .spawn(move || worker(received, code_rx));
            if let Err(e) = started {
                log::warn!("Ferrite signer: no worker thread: {e}");
                return None;
            }
            Some(Channels {
                commands: Mutex::new(commands),
                codes: Mutex::new(codes),
            })
        })
        .as_ref()
}

fn send(command: Command) {
    if let Some(channels) = channels() {
        let _ = channels
            .commands
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .send(command);
    }
}

/// Checks what a new signature for `bundle_id` (the running app's own) would be: team, App ID
/// and profile, without installing anything. Needs the Apple ID signed in.
pub fn check(bundle_id: &str) {
    send(Command::Check {
        bundle_id: bundle_id.to_owned(),
    });
}

/// Tries to reach the iPhone's installer with the pairing file at `pairing` (LocalDevVPN on):
/// the iPhone's name shows when it works. Needs no Apple ID.
pub fn test_connection(pairing: Option<&Path>) {
    send(Command::Test {
        pairing: pairing.map(Path::to_owned),
    });
}

/// Signs the app at `app` (the running one's bundle) again under `bundle_id` and installs it
/// over itself, with `work` as the folder for the signed copy. Needs the Apple ID signed in.
pub fn refresh(bundle_id: &str, app: &Path, work: &Path, pairing: Option<&Path>) {
    send(Command::Refresh {
        bundle_id: bundle_id.to_owned(),
        app: app.to_owned(),
        work: work.to_owned(),
        pairing: pairing.map(Path::to_owned),
    });
}

/// Downloads the release at `url` (its `.ipa`), signs it under `bundle_id` (the running app's)
/// and installs it over this app, as SideStore updates an app. Needs what Refresh now needs.
pub fn update(url: &str, bundle_id: &str, work: &Path, pairing: Option<&Path>) {
    send(Command::Update {
        url: url.to_owned(),
        bundle_id: bundle_id.to_owned(),
        work: work.to_owned(),
        pairing: pairing.map(Path::to_owned),
    });
}

/// Forgets the Apple session (the certificate's key stays in the Keychain for next time).
pub fn sign_out() {
    send(Command::SignOut);
}

/// Swift's secure sign-in form: the Apple ID and its password. The password is used for this
/// sign-in only.
#[unsafe(no_mangle)]
pub extern "C" fn hz_signer_sign_in(email: *const c_char, password: *const c_char) {
    if email.is_null() || password.is_null() {
        return;
    }
    // SAFETY: Swift passes NUL-terminated strings that live for the call; they are copied.
    let (email, password) = unsafe {
        (
            CStr::from_ptr(email).to_string_lossy().trim().to_owned(),
            CStr::from_ptr(password).to_string_lossy().into_owned(),
        )
    };
    if email.is_empty() || password.is_empty() {
        return set_status(Status::Failed("Enter your Apple ID and its password.".into()));
    }
    send(Command::SignIn { email, password });
}

/// The 2FA code the player typed in the launcher, or `#sms` (send the code by text instead) or
/// `#resend` (send it again).
#[unsafe(no_mangle)]
pub extern "C" fn hz_signer_code(code: *const c_char) {
    if code.is_null() {
        return;
    }
    // SAFETY: as `hz_signer_sign_in`.
    let code = unsafe { CStr::from_ptr(code) }.to_string_lossy().trim().to_owned();
    if let Some(channels) = channels() {
        let _ = channels.codes.lock().unwrap_or_else(|e| e.into_inner()).send(code);
        set_status(Status::SigningIn);
    }
}

/// The first line of an error, for the screen; the whole of it goes to the log.
fn short(e: &impl std::fmt::Display) -> String {
    let text = e.to_string();
    log::warn!("Ferrite signer: {text}");
    text.lines().next().unwrap_or("It didn't work.").trim().to_owned()
}

/// The worker: one Apple session at a time, its commands in order.
fn worker(commands: Receiver<Command>, codes: tokio::sync::mpsc::UnboundedReceiver<String>) {
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(e) => return set_status(Status::Failed(short(&e))),
    };
    let _ = isideload::init();
    let codes = std::sync::Arc::new(tokio::sync::Mutex::new(codes));
    let mut session: Option<(DeveloperSession, String)> = None;
    while let Ok(command) = commands.recv() {
        match command {
            Command::SignIn { email, password } => {
                set_status(Status::SigningIn);
                change(|s| s.email = Some(email.clone()));
                let codes = codes.clone();
                let signed = runtime.block_on(sign_in(&email, password, codes));
                match signed {
                    Ok((dev, team)) => {
                        change(|s| {
                            s.team = Some(team.name.clone().unwrap_or(team.team_id.clone()));
                            s.status = Status::SignedIn;
                        });
                        log::info!("Ferrite signer: signed in");
                        session = Some((dev, email));
                    }
                    Err(e) => set_status(Status::Failed(e)),
                }
            }
            Command::Check { bundle_id } => {
                let Some((dev, email)) = session.as_mut() else {
                    set_status(Status::Failed("Sign in with your Apple ID first.".into()));
                    continue;
                };
                set_status(Status::Working("Asking Apple for a new signature...".into()));
                match runtime.block_on(check_signature(dev, email, &bundle_id)) {
                    Ok(expires) => {
                        log::info!("Ferrite signer: a new signature for {bundle_id} would run until {expires}");
                        change(|s| {
                            s.checked = Some((bundle_id, expires));
                            s.status = Status::SignedIn;
                        });
                    }
                    Err(e) => set_status(Status::Failed(e)),
                }
            }
            Command::Test { pairing } => {
                let signed_in = session.is_some();
                let Some(pairing) = pairing else {
                    finish(signed_in, false, NO_PAIRING.into());
                    continue;
                };
                change(|s| s.refresh_note = None);
                step("Reaching your iPhone", 0.02);
                let reached = runtime.block_on(async {
                    match provider(&pairing) {
                        Ok(provider) => reach(&provider).await,
                        Err(e) => Err(e),
                    }
                });
                match reached {
                    Ok(device) => {
                        log::info!("Ferrite signer: reached {}", device.name);
                        let note = format!("Connected to {}", device.name);
                        change(|s| s.device = Some(device.name));
                        finish(signed_in, true, note);
                    }
                    Err(e) => finish(signed_in, false, e),
                }
            }
            Command::Refresh { bundle_id, app, work, pairing } => {
                let source = Source::Running(app);
                let note = "Installed. If Ferrite is still open, close it and open it again.";
                run_install(&runtime, session.as_mut(), &bundle_id, source, &work, pairing, note);
            }
            Command::Update { url, bundle_id, work, pairing } => {
                let source = Source::Download(url);
                let note = "Updated. If Ferrite is still open, close it and open it again.";
                run_install(&runtime, session.as_mut(), &bundle_id, source, &work, pairing, note);
            }
            Command::SignOut => {
                session = None;
                change(|s| *s = State::default());
            }
        }
    }
}

/// Refresh now or an update, from the worker: the Apple ID and pairing file there, then
/// `sign_and_install`, and the card's note.
fn run_install(
    runtime: &tokio::runtime::Runtime,
    session: Option<&mut (DeveloperSession, String)>,
    bundle_id: &str,
    source: Source,
    work: &Path,
    pairing: Option<PathBuf>,
    done_note: &str,
) {
    let Some((dev, email)) = session else {
        return finish(false, false, "Sign in with your Apple ID first.".into());
    };
    let Some(pairing) = pairing else {
        return finish(true, false, NO_PAIRING.into());
    };
    change(|s| s.refresh_note = None);
    let done = runtime.block_on(sign_and_install(dev, email, bundle_id, source, work, &pairing));
    match done {
        Ok(()) => {
            log::info!("Ferrite signer: installed");
            finish(true, true, done_note.to_owned());
        }
        Err(e) => finish(true, false, e),
    }
    let _ = std::fs::remove_dir_all(work);
}

/// The connection to the iPhone's installer, with the player's pairing file.
fn provider(pairing: &Path) -> Result<TcpProvider, String> {
    let pairing_file = PairingFile::read_from_file(pairing)
        .map_err(|e| format!("The pairing file can't be read ({})", short(&e)))?;
    Ok(TcpProvider {
        addr: DEVICE_ADDR,
        scope_id: None,
        pairing_file,
        label: "Ferrite".into(),
    })
}

/// The iPhone's name and UDID from its installer: proof the connection works.
async fn reach(provider: &TcpProvider) -> Result<IdeviceInfo, String> {
    tokio::time::timeout(CONNECT_WAIT, IdeviceInfo::from_device(provider))
        .await
        .map_err(|_| "No answer from your iPhone. Turn LocalDevVPN on and try again.".to_owned())?
        .map_err(|e| {
            format!(
                "Your iPhone didn't let Ferrite in ({}). Is LocalDevVPN on, and is the pairing file this iPhone's?",
                short(&e)
            )
        })
}

/// The team's App ID for `bundle_id`, added if this Apple ID doesn't have it yet.
async fn app_id_for(
    dev: &mut DeveloperSession,
    team: &DeveloperTeam,
    bundle_id: &str,
) -> Result<AppId, String> {
    let listed = dev.list_app_ids(team, None).await.map_err(|e| short(&e))?;
    match listed.app_ids.into_iter().find(|a| a.identifier == bundle_id) {
        Some(app_id) => Ok(app_id),
        None => dev
            .add_app_id(team, "Ferrite", bundle_id, None)
            .await
            .map_err(|e| short(&e)),
    }
}

/// The Apple ID's team (a free Apple ID has one: its Personal Team).
async fn first_team(dev: &mut DeveloperSession) -> Result<DeveloperTeam, String> {
    dev.list_teams()
        .await
        .map_err(|e| short(&e))?
        .into_iter()
        .next()
        .ok_or_else(|| "This Apple ID has no developer team.".to_owned())
}

/// `from` copied into `to`, folders and all.
fn copy_dir(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else {
            std::fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

/// What Refresh now signs (the running app's own bundle) or an update installs (a release's
/// `.ipa`, downloaded).
enum Source {
    Running(PathBuf),
    Download(String),
}

/// A release's `.ipa` saved at `to`, the download's progress shown (0.16 to 0.28).
fn download(url: &str, to: &Path) -> Result<(), String> {
    use std::io::{Read, Write};
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(15))
        .timeout_read(Duration::from_secs(60))
        .build();
    let response = agent
        .get(url)
        .call()
        .map_err(|e| format!("The update couldn't be downloaded ({e})"))?;
    let total = response
        .header("Content-Length")
        .and_then(|v| v.parse::<u64>().ok())
        .filter(|t| *t > 0);
    let mut reader = response.into_reader();
    let mut file =
        std::fs::File::create(to).map_err(|e| format!("The update couldn't be saved ({e})"))?;
    let mut buffer = vec![0u8; 1 << 16];
    let mut done = 0u64;
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|e| format!("The download stopped ({e})"))?;
        if read == 0 {
            break;
        }
        file.write_all(&buffer[..read])
            .map_err(|e| format!("The update couldn't be saved ({e}). Is the iPhone full?"))?;
        done += read as u64;
        if let Some(total) = total {
            let part = (done as f32 / total as f32).min(1.0);
            change(|s| s.progress = Some(0.16 + part * 0.12));
        }
    }
    Ok(())
}

/// Refresh now (`Source::Running`) or an update (`Source::Download`): reach the iPhone, register
/// it, get a new profile, sign the app under the installed one's bundle ID and install it over
/// the installed one.
async fn sign_and_install(
    dev: &mut DeveloperSession,
    email: &str,
    bundle_id: &str,
    source: Source,
    work: &Path,
    pairing: &Path,
) -> Result<(), String> {
    step("Reaching your iPhone", 0.02);
    let provider = provider(pairing)?;
    let device = reach(&provider).await?;
    change(|s| s.device = Some(device.name.clone()));
    let team = first_team(dev).await?;

    step("Making sure Apple knows this iPhone", 0.05);
    dev.ensure_device_registered(&team, &device.name, &device.udid, None)
        .await
        .map_err(|e| short(&e))?;

    step("Getting Ferrite's certificate", 0.08);
    let storage = new_storage();
    let certificate =
        CertificateIdentity::retrieve(MACHINE_NAME, email, dev, &team, &storage, &MaxCertsBehavior::Error)
            .await
            .map_err(|e| short(&e))?;

    step("Getting a new 7-day profile", 0.12);
    let app_id = app_id_for(dev, &team, bundle_id).await?;
    // More memory for higher render distances, where Apple allows it; Ferrite works without.
    if let Err(e) = dev.add_increased_memory_limit(&team, &app_id).await {
        log::warn!("Ferrite signer: no increased memory limit: {e}");
    }
    let profile = dev
        .download_team_provisioning_profile(&team, &app_id, None)
        .await
        .map_err(|e| short(&e))?;

    let _ = std::fs::remove_dir_all(work);
    std::fs::create_dir_all(work).map_err(|e| format!("No room to work in ({e})"))?;
    let updating = matches!(source, Source::Download(_));
    let path = match source {
        Source::Running(app) => {
            step("Copying Ferrite", 0.16);
            let name = app.file_name().map_or_else(|| "Ferrite.app".into(), |n| n.to_os_string());
            let copy = work.join(name);
            copy_dir(&app, &copy)
                .map_err(|e| format!("Ferrite couldn't be copied ({e}). Is the iPhone full?"))?;
            copy
        }
        Source::Download(url) => {
            step("Downloading the update", 0.16);
            let ipa = work.join("update.ipa");
            let (from, to) = (url.clone(), ipa.clone());
            tokio::task::spawn_blocking(move || download(&from, &to))
                .await
                .map_err(|e| e.to_string())??;
            ipa
        }
    };

    step("Signing", 0.3);
    // An `.ipa` is unpacked; a folder is the app itself.
    let mut signed = Application::new(path).map_err(|e| short(&e))?;
    // The installed app's own bundle ID (a release's has the original one): iOS updates the
    // installed app, worlds and sign-ins kept, instead of installing a second one.
    signed.bundle.set_bundle_identifier(bundle_id);
    signed.bundle.write_info().map_err(|e| short(&e))?;
    let app_dir = signed.bundle.bundle_dir.clone();
    // The new profile, and no old signature: the signer writes a new one.
    let profile_bytes: &[u8] = profile.encoded_profile.as_ref();
    std::fs::write(app_dir.join("embedded.mobileprovision"), profile_bytes)
        .map_err(|e| format!("The profile couldn't be saved ({e})"))?;
    let _ = std::fs::remove_dir_all(app_dir.join("_CodeSignature"));
    let special: Option<SpecialApp> = None;
    isideload::sideload::sign::sign(
        &mut signed,
        &certificate,
        &profile,
        &special,
        &team,
        Some(|done: f32| async move {
            // The signer counts from 0.3 to 1: here, 0.3 to 0.55.
            change(|s| s.progress = Some(0.3 + (done - 0.3).max(0.0) / 0.7 * 0.25));
        }),
    )
    .await
    .map_err(|e| short(&e))?;

    step(
        if updating {
            "Installing the update: Ferrite closes now and opens again updated"
        } else {
            "Installing: Ferrite closes now and opens again with 7 new days"
        },
        0.55,
    );
    isideload::sideload::install::install_app(&provider, &app_dir, |percent| {
        change(|s| s.progress = Some(0.55 + percent.min(100) as f32 / 100.0 * 0.45));
    })
    .await
    .map_err(|e| short(&e))?;
    Ok(())
}

/// Signs the Apple ID in, asking the launcher for the 2FA code when Apple wants one, and picks
/// its team (a free Apple ID has one: its Personal Team).
async fn sign_in(
    email: &str,
    password: String,
    codes: std::sync::Arc<tokio::sync::Mutex<tokio::sync::mpsc::UnboundedReceiver<String>>>,
) -> Result<(DeveloperSession, DeveloperTeam), String> {
    let ask_code = move |params: TwoFactorCallbackParams| {
        let codes = codes.clone();
        async move {
            // Apple doesn't know where to send it: to the player's devices.
            if params.unknown {
                return Ok(TwoFactorCallbackResponse::SendToDevices);
            }
            let first_number = params.numbers.first().cloned();
            set_status(Status::NeedsCode {
                last_error: params.last_error.clone(),
                sms_to: first_number.as_ref().map(|n| format!("the number ending in {}", n.last_two_digits)),
            });
            let mut codes = codes.lock().await;
            // A code typed for an earlier sign-in is no good now.
            while codes.try_recv().is_ok() {}
            let typed = tokio::time::timeout(CODE_WAIT, codes.recv()).await.ok().flatten();
            Ok(match typed.as_deref() {
                None => TwoFactorCallbackResponse::Abort,
                Some("#resend") => TwoFactorCallbackResponse::ResendCode,
                Some("#sms") => match first_number {
                    Some(number) => TwoFactorCallbackResponse::SendSms(number.id),
                    None => TwoFactorCallbackResponse::ResendCode,
                },
                Some(code) => TwoFactorCallbackResponse::SubmitCode(code.to_owned()),
            })
        }
    };
    let mut account = AppleAccount::builder(email)
        .login(&password, ask_code)
        .await
        .map_err(|e| short(&e))?;
    drop(password);
    let mut dev = DeveloperSession::from_account(&mut account)
        .await
        .map_err(|e| short(&e))?;
    let teams = dev.list_teams().await.map_err(|e| short(&e))?;
    let team = teams
        .into_iter()
        .next()
        .ok_or_else(|| "This Apple ID has no developer team.".to_owned())?;
    Ok((dev, team))
}

/// What a new signature for `bundle_id` would be: Ferrite's certificate (made the first time,
/// never by revoking another), the App ID (added if this Apple ID doesn't have it yet), and a
/// fresh profile. Returns when the profile runs out (Unix seconds).
async fn check_signature(
    dev: &mut DeveloperSession,
    email: &str,
    bundle_id: &str,
) -> Result<u64, String> {
    let team = dev
        .list_teams()
        .await
        .map_err(|e| short(&e))?
        .into_iter()
        .next()
        .ok_or_else(|| "This Apple ID has no developer team.".to_owned())?;
    let storage = new_storage();
    CertificateIdentity::retrieve(MACHINE_NAME, email, dev, &team, &storage, &MaxCertsBehavior::Error)
        .await
        .map_err(|e| short(&e))?;
    let listed = dev.list_app_ids(&team, None).await.map_err(|e| short(&e))?;
    let app_id = match listed.app_ids.into_iter().find(|a| a.identifier == bundle_id) {
        Some(app_id) => app_id,
        None => dev
            .add_app_id(&team, "Ferrite", bundle_id, None)
            .await
            .map_err(|e| short(&e))?,
    };
    let profile = dev
        .download_team_provisioning_profile(&team, &app_id, None)
        .await
        .map_err(|e| short(&e))?;
    let expires: SystemTime = profile.date_expire.into();
    Ok(expires
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs()))
}
