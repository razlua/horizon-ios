//! Startup shared by the gameplay library and its executable.
use crate::{crash, game, logging, ram_limit};

/// Start the game. The executable supplies the allocator and retains the mod API marker.
pub fn main() {
    main_with(Default::default());
}

#[cfg(target_os = "android")]
static ANDROID_APP: std::sync::OnceLock<winit::platform::android::activity::AndroidApp> =
    std::sync::OnceLock::new();

/// Android: the activity's app, which the event loop is made with and whose window flags keep
/// the screen on.
#[cfg(target_os = "android")]
pub fn android_app() -> Option<&'static winit::platform::android::activity::AndroidApp> {
    ANDROID_APP.get()
}

/// Android: the game, started by `android_main` (crates/horizon-android) on the thread the
/// activity's glue made for it. The game folder is already the current directory.
#[cfg(target_os = "android")]
pub fn android_main(app: winit::platform::android::activity::AndroidApp) {
    let _ = ANDROID_APP.set(app);
    main();
}

/// iOS: the app's own folder is read-only and the process starts in `/`. The game folder is the
/// app's Documents, which the Files app shows ("On My iPhone" > Horizon), and its home (logs,
/// crash reports) is `home` in it. The prepared resources are `resources` there: unpacked at the
/// first start from the `resources.zip` the player added to the app (tools/ios/add-resources.py),
/// or copied in with the Files app.
///
/// Returns the unpacking, when there is some to do: it takes too long to run before the event
/// loop starts (`game::ios_boot`).
#[cfg(target_os = "ios")]
fn ios_folders() -> Option<Box<dyn FnOnce() + Send>> {
    let container = std::env::var_os("HOME")?;
    let documents = std::path::Path::new(&container).join("Documents");
    let _ = std::fs::create_dir_all(&documents);
    let _ = std::env::set_current_dir(&documents);
    logging::set_game_home(documents.join("home"));
    let archive = std::env::current_exe().ok()?.parent()?.join("resources.zip");
    let meta = std::fs::metadata(&archive).ok()?;
    let modified = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_secs());
    // Names the archive: an updated app unpacks again.
    let stamp = format!("{} {modified}", meta.len());
    let target = documents.join("resources");
    if std::fs::read_to_string(target.join(".unpacked-from")).is_ok_and(|m| m == stamp) {
        return None;
    }
    Some(Box::new(move || {
        let started = std::time::Instant::now();
        match ios_unpack_resources(&archive, &documents, &stamp) {
            Ok(()) => {
                let _ = std::fs::remove_file(documents.join("ios-resources-error.txt"));
                log::info!(
                    "Horizon: unpacked the resources in {} ms",
                    started.elapsed().as_millis()
                );
            }
            // A file the Files app shows.
            Err(e) => {
                log::error!("Horizon: unpacking the resources failed: {e}");
                let _ = std::fs::write(
                    documents.join("ios-resources-error.txt"),
                    format!("Unpacking the game's resources failed: {e}\n"),
                );
            }
        }
    }))
}

/// iOS: what the app holds and what its Documents hold, for a bug report's log.
#[cfg(target_os = "ios")]
fn ios_log_bundle() {
    let exe = std::env::current_exe();
    let bundle = exe
        .as_ref()
        .ok()
        .and_then(|e| e.parent())
        .map(|p| p.to_path_buf());
    let archive = bundle
        .as_ref()
        .and_then(|b| std::fs::metadata(b.join("resources.zip")).ok())
        .map(|m| m.len());
    log::info!(
        "Horizon iOS: executable {exe:?}; resources.zip in the app: {}",
        archive.map_or_else(|| "none".to_owned(), |n| format!("{} MB", n / 1_000_000))
    );
    let listing = |dir: &std::path::Path| -> String {
        std::fs::read_dir(dir).map_or_else(
            |e| format!("({e})"),
            |entries| {
                let mut names: Vec<String> = entries
                    .filter_map(|e| e.ok())
                    .map(|e| e.file_name().to_string_lossy().into_owned())
                    .collect();
                names.sort();
                names.join(", ")
            },
        )
    };
    if let Some(bundle) = &bundle {
        log::info!("Horizon iOS: the app holds {}", listing(bundle));
    }
    if let Ok(cwd) = std::env::current_dir() {
        log::info!("Horizon iOS: {} holds {}", cwd.display(), listing(&cwd));
    }
}

/// Unpacks the app's `resources.zip` into `Documents/resources`, with a marker file naming the
/// archive it came from.
#[cfg(target_os = "ios")]
fn ios_unpack_resources(
    archive: &std::path::Path,
    documents: &std::path::Path,
    stamp: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let target = documents.join("resources");
    // Into a folder of its own first: a start cut short leaves no half-filled `resources`.
    let staging = documents.join("resources.unpacking");
    let _ = std::fs::remove_dir_all(&staging);
    let mut zip = zip::ZipArchive::new(std::io::BufReader::new(std::fs::File::open(archive)?))?;
    // File by file, counted for the start screen's progress bar (game::ios_boot::UNPACKED).
    use std::sync::atomic::Ordering::Relaxed;
    let [done, total] = &crate::game::ios_boot::UNPACKED;
    total.store(zip.len(), Relaxed);
    for i in 0..zip.len() {
        let mut file = zip.by_index(i)?;
        let Some(name) = file.enclosed_name() else {
            continue;
        };
        let path = staging.join(name);
        if file.is_dir() {
            std::fs::create_dir_all(&path)?;
        } else {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::io::copy(&mut file, &mut std::fs::File::create(&path)?)?;
        }
        done.store(i + 1, Relaxed);
    }
    std::fs::write(staging.join(".unpacked-from"), stamp)?;
    let _ = std::fs::remove_dir_all(&target);
    std::fs::rename(&staging, &target)?;
    Ok(())
}

/// Start the game with trait mods in dispatch order.
pub fn main_with(mods: crate::mod_api::Mods) {
    // First: a trait injection into anything below (logging, the crash handler) calls its mod.
    if crate::mod_api::MODS.set(mods).is_err() {
        log::warn!("Trait mods already installed; ignoring the second list");
    }
    #[cfg(target_os = "ios")]
    let unpacking = ios_folders();
    // Android: the game folder is the current directory (crates/horizon-android), its home
    // `home` in it as on iOS; the app carries no game files to unpack.
    #[cfg(target_os = "android")]
    let unpacking: Option<Box<dyn FnOnce() + Send>> = {
        if let Ok(folder) = std::env::current_dir() {
            logging::set_game_home(folder.join("home"));
        }
        None
    };
    horizon_threads::configure_allocator();
    horizon_client::telemetry::mark_start();
    horizon_client::spans::startup::init(std::time::Instant::now());
    let args: Vec<String> = std::env::args().skip(1).collect();
    let game_args = args.iter().take_while(|a| *a != "--dedicated");
    let help = game_args
        .clone()
        .any(|a| a == "--help" || a == "-h" || a == "--launch-api");
    // --home before the log opens in it.
    if let Some(dir) = game_args.clone().skip_while(|a| *a != "--home").nth(1) {
        logging::set_game_home(std::path::absolute(dir).unwrap_or_else(|_| dir.into()));
    }
    // The dedicated server logs in its own folder (--game-dir's), as vanilla's does; the game in its home.
    let (home, side, thread) = if args.iter().any(|a| a == "--dedicated") {
        let folder = game_args
            .clone()
            .skip_while(|a| *a != "--game-dir" && *a != "--gameDir")
            .nth(1)
            .and_then(|dir| std::path::absolute(dir).ok())
            .unwrap_or_else(|| ".".into());
        (folder, "server", "Server thread")
    } else {
        // --player-dir too (a game's, not a server's): a player of a shared game folder keeps
        // its log and crash reports in its own; crash telemetry stays with the home's consent.
        if let Some(dir) = game_args
            .clone()
            .skip_while(|a| *a != "--player-dir")
            .nth(1)
        {
            logging::set_player_dir(std::path::absolute(dir).unwrap_or_else(|_| dir.into()));
        }
        (logging::game_home(), "client", "Render thread")
    };
    if !help {
        logging::init(
            logging::player_dir_override().unwrap_or(home.as_path()),
            thread,
        );
        // The log's first line names the build, so a bug report that carries the log says which one it was.
        log::info!("Horizon {}", crash::version());
        #[cfg(target_os = "ios")]
        ios_log_bundle();
        crash::install(home.clone(), side);
        ram_limit::start(&home, side);
    }
    let renderers = crate::mod_api::mods()
        .iter()
        .filter_map(|m| m.render())
        .collect();
    if crate::gfx::native_render::install(renderers).is_err() {
        log::warn!("Trait renderers already installed; keeping the first list");
    }
    if horizon_world::native_load_hooks::install(
        horizon_world::native_load_hooks::NativeLoadHooks {
            loot_table_load: |id, table| {
                crate::mod_api::dispatch_loot_table_load(crate::mod_api::mods(), id, table);
            },
            modify_default_components: |item, components| {
                crate::mod_api::dispatch_modify_default_components(
                    crate::mod_api::mods(),
                    item,
                    components,
                );
            },
            loot_function: |kind, data, ctx, stack| {
                crate::mod_api::dispatch_loot_function(
                    crate::mod_api::mods(),
                    kind,
                    data,
                    ctx,
                    stack,
                );
            },
            loot_condition: |kind, data, ctx, holds| {
                crate::mod_api::dispatch_loot_condition(
                    crate::mod_api::mods(),
                    kind,
                    data,
                    ctx,
                    holds,
                );
            },
        },
    )
    .is_err()
    {
        log::warn!("Native data-load hooks already installed; keeping the first callbacks");
    }
    if crate::native_client_hooks::install(crate::native_client_hooks::NativeClientHooks {
        menu_slot_rules: |menu, slot, stack, may_place, max_stack| {
            crate::mod_api::dispatch_menu_slot_rules(
                crate::mod_api::mods(),
                menu,
                slot,
                stack,
                may_place,
                max_stack,
            );
        },
        player_chat: |player, name, message, replies, cancel| {
            crate::mod_api::dispatch_player_chat(
                crate::mod_api::mods(),
                player,
                name,
                message,
                replies,
                cancel,
            );
        },
        living_heal: |entity, amount, cancel| {
            crate::mod_api::dispatch_living_heal(crate::mod_api::mods(), entity, amount, cancel);
        },
        living_breathe: |entity, kind, can_breathe, consume, refill| {
            crate::mod_api::dispatch_living_breathe(
                crate::mod_api::mods(),
                entity,
                kind,
                can_breathe,
                consume,
                refill,
            );
        },
        living_drown: |entity, kind, is_drowning, damage_amount, bubble_count, cancel| {
            crate::mod_api::dispatch_living_drown(
                crate::mod_api::mods(),
                entity,
                kind,
                is_drowning,
                damage_amount,
                bubble_count,
                cancel,
            );
        },
        living_fall: |entity, kind, distance, multiplier, cancel| {
            crate::mod_api::dispatch_living_fall(
                crate::mod_api::mods(),
                entity,
                kind,
                distance,
                multiplier,
                cancel,
            );
        },
        living_jump: |entity, kind| {
            crate::mod_api::dispatch_living_jump(crate::mod_api::mods(), entity, kind);
        },
        ingredient: |kind, data, items| {
            crate::mod_api::dispatch_ingredient(crate::mod_api::mods(), kind, data, items);
        },
        ingredient_test: |kind, data, stack, accept| {
            crate::mod_api::dispatch_ingredient_test(
                crate::mod_api::mods(),
                kind,
                data,
                stack,
                accept,
            );
        },
    })
    .is_err()
    {
        log::warn!("Native client hooks already installed; keeping the first callbacks");
    }
    raise_open_file_limit();
    #[cfg(any(target_os = "ios", target_os = "android"))]
    let ran = {
        // No files in the app and none prepared: the start screen downloads them (ios_setup.rs).
        let setup = unpacking.is_none()
            && !std::path::Path::new("resources/horizon-resources.json").is_file();
        game::ios_boot::run_after(unpacking, setup)
    };
    #[cfg(not(any(target_os = "ios", target_os = "android")))]
    let ran = game::run_with();
    if let Err(error) = ran {
        // A bad command line is the caller's mistake, not a crash: no report.
        let usage = error.is::<game::UsageError>();
        log::error!("Horizon: {error}");
        logging::flush();
        // The report's path is the last line on stderr (crash::SAVED_LINE).
        if !help && !usage {
            crash::report_error(&home, side, &*error);
        }
        // Started from Explorer, the console window closes with the game:
        // keep the message on screen until it is read.
        #[cfg(windows)]
        if std::io::IsTerminal::is_terminal(&std::io::stdin()) {
            eprintln!("Press Enter to close.");
            let _ = std::io::stdin().read_line(&mut String::new());
        }
        std::process::exit(if usage {
            game::exit_code::USAGE
        } else {
            game::exit_code::ERROR
        });
    }
    logging::flush();
}

/// The JVM raises its open-file limit to the hard limit as it starts
/// (`-XX:+MaxFDLimit`, on by default), so a vanilla server may hold as many
/// sockets and region files as the host allows, not the soft default (often
/// 1024 in a container). The same here; macOS caps it at `OPEN_MAX`, as the
/// JVM does there.
fn raise_open_file_limit() {
    #[cfg(unix)]
    {
        use rustix::process::{Resource, Rlimit, getrlimit, setrlimit};
        let limit = getrlimit(Resource::Nofile);
        let hard = if cfg!(target_os = "macos") {
            Some(limit.maximum.map_or(10_240, |h| h.min(10_240)))
        } else {
            limit.maximum
        };
        let raised = match (limit.current, hard) {
            (Some(soft), Some(hard)) => hard > soft,
            (Some(_), None) => true,
            (None, _) => false,
        };
        if raised {
            let to = Rlimit {
                current: hard,
                maximum: limit.maximum,
            };
            match setrlimit(Resource::Nofile, to) {
                Ok(()) => log::info!(
                    "Open file limit raised from {:?} to {hard:?}",
                    limit.current
                ),
                Err(e) => log::warn!("Open file limit stays at {:?}: {e}", limit.current),
            }
        }
    }
}
