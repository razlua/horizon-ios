//! iOS: the Microsoft account, signed in by the game itself. There is no launcher on iOS to hand
//! the game a session (docs/microsoft-account.md), so the app runs the launcher's flow: a device
//! code for the Microsoft account, then Xbox Live, XSTS and the Minecraft services login.
//!
//! Only the Microsoft refresh token, the player's name and uuid are kept, in the app's own
//! `Library/Application Support/horizon/account.json` (not in Documents, which the Files app
//! shows), readable by its owner only. The Minecraft access token lives in memory.

use serde_json::{Value, json};
use std::path::PathBuf;
use std::time::Duration;

/// Cazza Launcher's Azure application: personal Microsoft accounts, public client flow, no
/// secret, approved for the Minecraft services.
const CLIENT_ID: &str = "967c7b5b-8b53-4975-b4e3-7c9a394ed3af";
const SCOPE: &str = "XboxLive.signin offline_access";
const DEVICE_CODE_URL: &str = "https://login.microsoftonline.com/consumers/oauth2/v2.0/devicecode";
const TOKEN_URL: &str = "https://login.microsoftonline.com/consumers/oauth2/v2.0/token";
const XBL_URL: &str = "https://user.auth.xboxlive.com/user/authenticate";
const XSTS_URL: &str = "https://xsts.auth.xboxlive.com/xsts/authorize";
const MC_LOGIN_URL: &str = "https://api.minecraftservices.com/authentication/login_with_xbox";
const MC_PROFILE_URL: &str = "https://api.minecraftservices.com/minecraft/profile";
const MC_ENTITLEMENTS_URL: &str = "https://api.minecraftservices.com/entitlements/mcstore";

/// A code to enter at Microsoft's page.
#[derive(Clone, Debug)]
pub struct DeviceCode {
    device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: Duration,
    pub interval: Duration,
}

impl DeviceCode {
    /// Microsoft's page with the code already filled in.
    pub fn link(&self) -> String {
        format!("https://www.microsoft.com/link?otc={}", self.user_code)
    }
}

/// One look at whether the player finished at Microsoft's page.
pub enum Poll {
    Pending,
    /// Asked to wait longer between looks.
    SlowDown,
    /// The Microsoft access and refresh tokens.
    Done(String, String),
}

/// A signed-in player.
pub struct Signed {
    pub name: String,
    /// Without dashes.
    pub uuid: String,
    pub access_token: String,
    pub refresh_token: String,
}

impl Signed {
    /// The session the game reads (`AuthSession::parse`).
    pub fn session_json(&self) -> String {
        json!({
            "name": self.name,
            "uuid": self.uuid,
            "access_token": self.access_token,
            "user_type": "msa",
        })
        .to_string()
    }
}

/// An account kept from a sign-in.
#[derive(Clone, Debug)]
pub struct Saved {
    pub name: String,
    /// Without dashes.
    pub uuid: String,
    refresh_token: String,
}

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(20))
        .user_agent(&format!("Horizon/{} (iOS)", horizon_core::release::VERSION))
        .build()
}

/// The status and JSON body of an answer, whatever its status.
fn answer(result: Result<ureq::Response, ureq::Error>) -> Result<(u16, Value), String> {
    let response = match result {
        Ok(r) => r,
        Err(ureq::Error::Status(_, r)) => r,
        Err(e) => return Err(format!("no connection to the sign-in service ({e})")),
    };
    let status = response.status();
    let text = response.into_string().unwrap_or_default();
    Ok((status, serde_json::from_str(&text).unwrap_or(Value::Null)))
}

fn post_form(url: &str, form: &[(&str, &str)]) -> Result<(u16, Value), String> {
    answer(agent().post(url).send_form(form))
}

fn post_json(url: &str, body: &Value) -> Result<(u16, Value), String> {
    answer(
        agent()
            .post(url)
            .set("Content-Type", "application/json")
            .set("Accept", "application/json")
            .send_string(&body.to_string()),
    )
}

fn get_bearer(url: &str, token: &str) -> Result<(u16, Value), String> {
    answer(
        agent()
            .get(url)
            .set("Authorization", &format!("Bearer {token}"))
            .call(),
    )
}

fn oauth_error(body: &Value) -> String {
    body.get("error_description")
        .or_else(|| body.get("error"))
        .and_then(Value::as_str)
        .map(|s| s.lines().next().unwrap_or(s).to_owned())
        .unwrap_or_else(|| "unknown error".into())
}

fn text(body: &Value, key: &str) -> Option<String> {
    body.get(key).and_then(Value::as_str).map(str::to_owned)
}

/// Asks Microsoft for a code.
pub fn begin() -> Result<DeviceCode, String> {
    let (status, body) = post_form(DEVICE_CODE_URL, &[("client_id", CLIENT_ID), ("scope", SCOPE)])?;
    if status != 200 {
        return Err(format!("Microsoft refused the sign-in: {}", oauth_error(&body)));
    }
    let field = |k| text(&body, k).ok_or_else(|| format!("Microsoft's answer had no {k}"));
    Ok(DeviceCode {
        device_code: field("device_code")?,
        user_code: field("user_code")?,
        verification_uri: field("verification_uri")?,
        expires_in: Duration::from_secs(body["expires_in"].as_u64().unwrap_or(900)),
        interval: Duration::from_secs(body["interval"].as_u64().unwrap_or(5).max(1)),
    })
}

/// Whether the player finished at Microsoft's page.
pub fn poll(code: &DeviceCode) -> Result<Poll, String> {
    let (status, body) = post_form(
        TOKEN_URL,
        &[
            ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
            ("client_id", CLIENT_ID),
            ("device_code", &code.device_code),
        ],
    )?;
    if status == 200 {
        let access = text(&body, "access_token").ok_or("Microsoft sent no access token")?;
        let refresh = text(&body, "refresh_token").ok_or("Microsoft sent no refresh token")?;
        return Ok(Poll::Done(access, refresh));
    }
    match body.get("error").and_then(Value::as_str).unwrap_or("") {
        "authorization_pending" => Ok(Poll::Pending),
        "slow_down" => Ok(Poll::SlowDown),
        "authorization_declined" => Err("The sign-in was declined.".into()),
        "expired_token" => Err("The code expired.".into()),
        _ => Err(format!("Microsoft sign-in failed: {}", oauth_error(&body))),
    }
}

/// New Microsoft tokens for a kept refresh token.
fn refresh(refresh_token: &str) -> Result<(String, String), String> {
    let (status, body) = post_form(
        TOKEN_URL,
        &[
            ("grant_type", "refresh_token"),
            ("client_id", CLIENT_ID),
            ("refresh_token", refresh_token),
            ("scope", SCOPE),
        ],
    )?;
    if status != 200 {
        return Err(format!("The saved sign-in expired: {}", oauth_error(&body)));
    }
    let access = text(&body, "access_token").ok_or("Microsoft sent no access token")?;
    let refresh = text(&body, "refresh_token").unwrap_or_else(|| refresh_token.to_owned());
    Ok((access, refresh))
}

/// Xbox Live, XSTS, then the Minecraft services: the Minecraft token and profile.
pub fn minecraft(msa_access: &str, refresh_token: String) -> Result<Signed, String> {
    let (status, xbl) = post_json(
        XBL_URL,
        &json!({
            "Properties": {
                "AuthMethod": "RPS",
                "SiteName": "user.auth.xboxlive.com",
                "RpsTicket": format!("d={msa_access}"),
            },
            "RelyingParty": "http://auth.xboxlive.com",
            "TokenType": "JWT",
        }),
    )?;
    if status != 200 {
        return Err(format!("Xbox Live refused the sign-in ({status})."));
    }
    let xbl_token = text(&xbl, "Token").ok_or("Xbox Live sent no token")?;
    let (status, xsts) = post_json(
        XSTS_URL,
        &json!({
            "Properties": { "SandboxId": "RETAIL", "UserTokens": [xbl_token] },
            "RelyingParty": "rp://api.minecraftservices.com/",
            "TokenType": "JWT",
        }),
    )?;
    if status == 401 {
        return Err(match xsts["XErr"].as_u64().unwrap_or(0) {
            2148916233 => "This Microsoft account has no Xbox profile: sign in at minecraft.net once.",
            2148916235 => "Xbox Live is not available in your country or region.",
            2148916236 | 2148916237 => "This account needs adult verification on the Xbox website.",
            2148916238 => "This is a child account: an adult must add it to a Microsoft family first.",
            _ => "Xbox Live refused the sign-in (XSTS).",
        }
        .into());
    }
    if status != 200 {
        return Err(format!("Xbox Live refused the sign-in (XSTS {status})."));
    }
    let xsts_token = text(&xsts, "Token").ok_or("Xbox Live sent no XSTS token")?;
    let uhs = xsts["DisplayClaims"]["xui"][0]["uhs"]
        .as_str()
        .ok_or("Xbox Live sent no user hash")?;
    let (status, login) = post_json(
        MC_LOGIN_URL,
        &json!({ "identityToken": format!("XBL3.0 x={uhs};{xsts_token}") }),
    )?;
    if status == 403 {
        return Err("The Minecraft services refused this app's sign-in (403).".into());
    }
    if status != 200 {
        return Err(format!("The Minecraft services refused the sign-in ({status})."));
    }
    let access_token = text(&login, "access_token").ok_or("Minecraft sent no access token")?;
    let (status, profile) = get_bearer(MC_PROFILE_URL, &access_token)?;
    if status == 404 {
        let owns = get_bearer(MC_ENTITLEMENTS_URL, &access_token)
            .ok()
            .and_then(|(_, e)| e["items"].as_array().map(|a| !a.is_empty()))
            .unwrap_or(false);
        return Err(if owns {
            "Choose your Minecraft name at minecraft.net first."
        } else {
            "This Microsoft account does not own Minecraft: Java Edition."
        }
        .into());
    }
    if status != 200 {
        return Err(format!("The Minecraft profile could not be read ({status})."));
    }
    Ok(Signed {
        name: text(&profile, "name").ok_or("The Minecraft profile has no name")?,
        uuid: text(&profile, "id").ok_or("The Minecraft profile has no id")?,
        access_token,
        refresh_token,
    })
}

/// Signs the kept account in again, without the player.
pub fn resume(saved: &Saved) -> Result<Signed, String> {
    let (access, refresh_token) = refresh(&saved.refresh_token)?;
    minecraft(&access, refresh_token)
}

fn dir() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(PathBuf::from(home).join("Library/Application Support/horizon"))
}

/// The accounts, the one played last first.
fn file() -> Option<PathBuf> {
    Some(dir()?.join("accounts.json"))
}

/// The one account the first builds kept.
fn old_file() -> Option<PathBuf> {
    Some(dir()?.join("account.json"))
}

fn parse(entry: &Value) -> Option<Saved> {
    Some(Saved {
        name: text(entry, "name")?,
        uuid: text(entry, "uuid").unwrap_or_default(),
        refresh_token: text(entry, "refresh_token")?,
    })
}

/// The saved accounts, the one played last first. An account kept by an earlier build is one.
pub fn saved() -> Vec<Saved> {
    let read = |path: Option<PathBuf>| -> Option<Value> {
        serde_json::from_str(&std::fs::read_to_string(path?).ok()?).ok()
    };
    if let Some(body) = read(file()) {
        return body["accounts"]
            .as_array()
            .map(|list| list.iter().filter_map(parse).collect())
            .unwrap_or_default();
    }
    read(old_file()).as_ref().and_then(parse).into_iter().collect()
}

fn write(accounts: &[Saved]) {
    let Some(path) = file() else {
        return;
    };
    let list: Vec<Value> = accounts
        .iter()
        .map(|a| json!({ "name": a.name, "uuid": a.uuid, "refresh_token": a.refresh_token }))
        .collect();
    let body = json!({ "accounts": list }).to_string();
    let written = path
        .parent()
        .map_or(Ok(()), std::fs::create_dir_all)
        .and_then(|()| {
            use std::io::Write;
            #[cfg(unix)]
            use std::os::unix::fs::OpenOptionsExt;
            let mut options = std::fs::OpenOptions::new();
            options.write(true).create(true).truncate(true);
            #[cfg(unix)]
            options.mode(0o600);
            options.open(&path)?.write_all(body.as_bytes())
        });
    match written {
        Ok(()) => {
            if let Some(old) = old_file() {
                let _ = std::fs::remove_file(old);
            }
        }
        Err(e) => log::warn!("Horizon: the accounts could not be kept for the next start: {e}"),
    }
}

/// Keeps a signed-in account for the next starts, first in the list.
pub fn save(signed: &Signed) {
    let mut accounts = saved();
    accounts.retain(|a| a.uuid != signed.uuid && a.name != signed.name);
    accounts.insert(
        0,
        Saved {
            name: signed.name.clone(),
            uuid: signed.uuid.clone(),
            refresh_token: signed.refresh_token.clone(),
        },
    );
    write(&accounts);
}

/// Forgets a saved account (signed out, or its sign-in no longer works).
pub fn forget(account: &Saved) {
    let mut accounts = saved();
    accounts.retain(|a| !(a.uuid == account.uuid && a.name == account.name));
    write(&accounts);
}
