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
#![cfg(target_os = "ios")]

use std::ffi::{CStr, c_char};
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, SystemTime};

use isideload::auth::apple_account::{
    AppleAccount, TwoFactorCallbackParams, TwoFactorCallbackResponse,
};
use isideload::dev::app_ids::AppIdsApi;
use isideload::dev::developer_session::DeveloperSession;
use isideload::dev::teams::{DeveloperTeam, TeamsApi};
use isideload::sideload::builder::MaxCertsBehavior;
use isideload::sideload::cert_identity::CertificateIdentity;
use isideload::util::storage::new_storage;

/// The name Ferrite's signing certificate carries in the Apple ID's developer account.
const MACHINE_NAME: &str = "Ferrite";
/// How long the 2FA code is waited for.
const CODE_WAIT: Duration = Duration::from_secs(300);

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
}

static STATE: Mutex<Option<State>> = Mutex::new(None);
static VERSION: AtomicU64 = AtomicU64::new(1);

enum Command {
    SignIn { email: String, password: String },
    Check { bundle_id: String },
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
    })
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
            Command::SignOut => {
                session = None;
                change(|s| *s = State::default());
            }
        }
    }
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
