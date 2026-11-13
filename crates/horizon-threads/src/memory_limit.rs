//! The player's cap on the memory the game holds, and the last resort behind it.
//!
//! On every system the game keeps itself under the cap by holding less, before it reaches it
//! (`horizon_client::ram_limit`, which reads [`pressure`]). Nothing asks the system to page the
//! game out: on Windows a hard working set limit did, and a low cap made the game slow instead of
//! making it hold less.
//!
//! A watcher thread reads the process's resident memory and, over the cap, gives free pages back.
//! Only when the game has nothing left to give up ([`can_shrink`] has stopped being called) and
//! stays over for [`GRACE`] does it call the game's `exceeded`. No thread runs until a cap is set,
//! and there is no lowest cap: one the game can't live under ends the game.
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Once, OnceLock};
use std::time::{Duration, Instant};

const NONE: u64 = u64::MAX;
const EVERY: Duration = Duration::from_secs(1);
/// How long the game may stay over the cap with nothing left to give up: a world's first chunks,
/// or a burst the last step down has not freed yet, pass within it.
pub const GRACE: Duration = Duration::from_secs(20);
/// How long one [`can_shrink`] counts for.
const SHRINK_WORD: Duration = Duration::from_secs(5);

static LIMIT: AtomicU64 = AtomicU64::new(NONE);
static WATCHER: Once = Once::new();
static STARTED: OnceLock<Instant> = OnceLock::new();
/// Until when, in milliseconds since [`STARTED`], the game says it can still shrink.
static SHRINKS_UNTIL: AtomicU64 = AtomicU64::new(0);

fn since_start() -> u64 {
    STARTED.get_or_init(Instant::now).elapsed().as_millis() as u64
}

/// The game can still hold less. Said again every second while that is true, so it ends by itself
/// when the world closes.
pub fn can_shrink() {
    let until = since_start() + SHRINK_WORD.as_millis() as u64;
    SHRINKS_UNTIL.store(until, Ordering::Relaxed);
}

/// Sets the cap in bytes, or removes it. `exceeded(used, limit)` runs on the watcher thread when
/// the game can't get under the cap; only the first call's function is kept.
pub fn set(limit: Option<u64>, exceeded: fn(u64, u64)) {
    LIMIT.store(limit.unwrap_or(NONE), Ordering::Relaxed);
    if limit.is_some() {
        WATCHER.call_once(|| {
            let watcher = std::thread::Builder::new()
                .name("Memory limit".into())
                .spawn(move || {
                    let mut over = Over::default();
                    loop {
                        std::thread::sleep(EVERY);
                        let limit = get();
                        over.follow(limit);
                        if let Some(limit) = limit {
                            check(&mut over, limit, exceeded);
                        }
                    }
                });
            if let Err(e) = watcher {
                eprintln!("The memory limit can't be watched: {e}");
            }
        });
    }
}

/// The cap in bytes, if one is set.
pub fn get() -> Option<u64> {
    Some(LIMIT.load(Ordering::Relaxed)).filter(|limit| *limit != NONE)
}

/// How tight memory is for the game.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Pressure {
    /// It should hold less; `keep` is the share of what it holds to aim for.
    Over {
        keep: f64,
    },
    /// It could hold more.
    Room,
    Steady,
}

/// What the game holds against the cap: it should act before it gets there.
fn pressure_of_usage(used: u64, limit: u64) -> Pressure {
    if used > limit / 10 * 9 {
        Pressure::Over {
            keep: limit as f64 * 0.8 / used as f64,
        }
    } else if used < limit / 10 * 7 {
        Pressure::Room
    } else {
        Pressure::Steady
    }
}

/// Against a hard ceiling, what the process commits is the budget, not what stays in RAM: a
/// ceiling counts every page the process committed, the renderer's own included.
pub fn pressure_of_commit(commit: u64, limit: u64) -> Pressure {
    if commit > limit / 10 * 9 {
        Pressure::Over {
            keep: limit as f64 * 0.8 / commit as f64,
        }
    } else if commit < limit / 10 * 7 {
        Pressure::Room
    } else {
        Pressure::Steady
    }
}

/// How tight memory is under `limit`: what the game holds in RAM against it. iOS: the room the
/// system says is left ([`pressure_of_headroom`]) decides, since that is what it ends apps by.
pub fn pressure(limit: u64) -> Pressure {
    #[cfg(target_os = "ios")]
    if let (Some(available), Some(used)) = (available_bytes(), resident_bytes()) {
        return pressure_of_headroom(available, used);
    }
    resident_bytes().map_or(Pressure::Steady, |used| pressure_of_usage(used, limit))
}

const MIB: u64 = 1024 * 1024;
/// Less room than this left before the system ends the app: hold less.
pub const HEADROOM_LOW: u64 = 600 * MIB;
/// More room than this left: the game may hold more again. Close to `HEADROOM_LOW`: a phone may
/// have only about 1 GB beyond what the game holds at any distance.
pub const HEADROOM_ROOMY: u64 = 880 * MIB;
/// The room a step down aims to leave.
const HEADROOM_AIM: u64 = 760 * MIB;
/// Less than this left: the system is about to end the app, so a big step at once.
const HEADROOM_URGENT: u64 = 320 * MIB;

/// How tight memory is when the system says how much room is left (`available`) and the game
/// holds `used`. A share of a fixed cap doesn't work where much of what the game holds (its game
/// data, textures, sounds) stays at any render distance: on an iPhone that allows 3.3 GB the game
/// held 2.3 GB at render distance 2, so a cap at 80% stepped down to 2 and never back up.
pub fn pressure_of_headroom(available: u64, used: u64) -> Pressure {
    if available < HEADROOM_LOW {
        let shed = HEADROOM_AIM - available;
        let keep = if used == 0 {
            0.8
        } else {
            used.saturating_sub(shed) as f64 / used as f64
        };
        let keep = if available < HEADROOM_URGENT { 0.5 } else { keep };
        Pressure::Over { keep: keep.clamp(0.5, 0.95) }
    } else if available > HEADROOM_ROOMY {
        Pressure::Room
    } else {
        Pressure::Steady
    }
}

#[cfg(test)]
mod headroom_tests {
    use super::*;

    #[test]
    fn room_left_decides_and_a_big_fixed_share_still_climbs_back() {
        let gb = |g: f64| (g * 1024.0) as u64 * MIB;
        // The iPhone of the report: 3.3 GB allowed, 2.3 GB held at distance 2, 1 GB left.
        assert_eq!(pressure_of_headroom(gb(1.05), gb(2.3)), Pressure::Room);
        assert_eq!(pressure_of_headroom(gb(0.8), gb(2.55)), Pressure::Steady);
        match pressure_of_headroom(gb(0.4), gb(2.95)) {
            Pressure::Over { keep } => assert!(keep > 0.8 && keep < 0.95, "{keep}"),
            other => panic!("{other:?}"),
        }
        // Almost nothing left: a big step, though not below half.
        match pressure_of_headroom(0, gb(1.0)) {
            Pressure::Over { keep } => assert_eq!(keep, 0.5),
            other => panic!("{other:?}"),
        }
    }
}

/// How long the game has been over the cap with nothing left to give up.
#[derive(Default)]
struct Over {
    since: Option<Instant>,
    /// The cap the grace is counted against.
    limit: Option<u64>,
}

impl Over {
    /// A cap turned off, or set to another value, starts its grace again.
    fn follow(&mut self, limit: Option<u64>) {
        if limit != self.limit {
            self.since = None;
            self.limit = limit;
        }
    }
    /// Whether this reading ends the game.
    fn gave_up(&mut self, now: Instant, over: bool, can_shrink: bool) -> bool {
        if !over || can_shrink {
            self.since = None;
            return false;
        }
        now.duration_since(*self.since.get_or_insert(now)) >= GRACE
    }
}

fn check(state: &mut Over, limit: u64, exceeded: fn(u64, u64)) {
    // A hard ceiling counts what the process committed, the renderer's own
    // pages included, since that is the number the player set.
    let hard = crate::capped::limit().is_some();
    let measured = if hard {
        Some(crate::allocator_commit().0 as u64)
    } else {
        resident_bytes()
    };
    let Some(mut used) = measured else {
        return;
    };
    if used > limit {
        crate::release_free_memory();
        used = if hard {
            crate::allocator_commit().0 as u64
        } else {
            resident_bytes().unwrap_or(used)
        };
    }
    let can_shrink = since_start() < SHRINKS_UNTIL.load(Ordering::Relaxed);
    if state.gave_up(Instant::now(), used > limit, can_shrink) {
        exceeded(used, limit);
    }
}

/// The process's memory counters.
#[cfg(windows)]
fn counters() -> Option<windows_sys::Win32::System::ProcessStatus::PROCESS_MEMORY_COUNTERS> {
    use windows_sys::Win32::System::ProcessStatus::{
        K32GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS,
    };
    use windows_sys::Win32::System::Threading::GetCurrentProcess;
    let size = std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
    // SAFETY: all-zero is a valid value of this plain C struct, and the call writes at most `size`
    // bytes into it; the current process's pseudo-handle needs no closing.
    unsafe {
        let mut counters: PROCESS_MEMORY_COUNTERS = std::mem::zeroed();
        counters.cb = size;
        (K32GetProcessMemoryInfo(GetCurrentProcess(), &mut counters, size) != 0).then_some(counters)
    }
}

/// The memory this process holds in RAM, in bytes.
#[cfg(windows)]
pub fn resident_bytes() -> Option<u64> {
    Some(counters()?.WorkingSetSize as u64)
}

/// The memory this process holds in RAM, in bytes.
#[cfg(any(target_os = "linux", target_os = "android"))]
pub fn resident_bytes() -> Option<u64> {
    let statm = std::fs::read_to_string("/proc/self/statm").ok()?;
    let pages: u64 = statm.split_whitespace().nth(1)?.parse().ok()?;
    // SAFETY: reads one system constant.
    let page = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
    Some(pages * u64::try_from(page).ok()?)
}

/// The memory this process holds in RAM, in bytes.
#[cfg(target_os = "macos")]
pub fn resident_bytes() -> Option<u64> {
    let size = std::mem::size_of::<libc::proc_taskinfo>() as libc::c_int;
    // SAFETY: all-zero is a valid value of this plain C struct, and the call writes at most `size`
    // bytes into it.
    unsafe {
        let mut info: libc::proc_taskinfo = std::mem::zeroed();
        let written = libc::proc_pidinfo(
            libc::getpid(),
            libc::PROC_PIDTASKINFO,
            0,
            (&raw mut info).cast(),
            size,
        );
        (written == size).then_some(info.pti_resident_size)
    }
}

/// iOS: the memory the system counts against the app, in bytes: its physical footprint (the
/// GPU's buffers and textures included), the number iOS ends an app for, not its resident size.
#[cfg(target_os = "ios")]
pub fn resident_bytes() -> Option<u64> {
    // `rusage_info_v0` (<sys/resource.h>); the libc crate declares the call for macOS only.
    #[repr(C)]
    struct Usage {
        uuid: [u8; 16],
        user_time: u64,
        system_time: u64,
        pkg_idle_wkups: u64,
        interrupt_wkups: u64,
        pageins: u64,
        wired_size: u64,
        resident_size: u64,
        phys_footprint: u64,
        proc_start_abstime: u64,
        proc_exit_abstime: u64,
    }
    unsafe extern "C" {
        // <libproc.h>, iOS 7 and later.
        fn proc_pid_rusage(pid: libc::c_int, flavor: libc::c_int, buffer: *mut Usage)
        -> libc::c_int;
    }
    const RUSAGE_INFO_V0: libc::c_int = 0;
    // SAFETY: all-zero is a valid value of this plain C struct, which is the layout the V0 flavor
    // writes, and the call writes only into it.
    unsafe {
        let mut usage: Usage = std::mem::zeroed();
        (proc_pid_rusage(libc::getpid(), RUSAGE_INFO_V0, &raw mut usage) == 0)
            .then_some(usage.phys_footprint)
    }
}

/// iOS: how much the app may hold before the system ends it, in bytes: what it holds now and
/// what `os_proc_available_memory` says is left. None where the system doesn't say.
#[cfg(target_os = "ios")]
pub fn system_limit() -> Option<u64> {
    available_bytes().map(|available| resident_bytes().unwrap_or(0) + available)
}

/// iOS: how much more the app may hold before the system ends it (`os_proc_available_memory`),
/// in bytes. None where the system doesn't say.
#[cfg(target_os = "ios")]
pub fn available_bytes() -> Option<u64> {
    unsafe extern "C" {
        // <os/proc.h>, iOS 13 and later.
        fn os_proc_available_memory() -> usize;
    }
    // SAFETY: takes nothing and returns a number.
    let available = unsafe { os_proc_available_memory() } as u64;
    (available > 0).then_some(available)
}

/// The memory this process holds in RAM, in bytes: not known on this system.
#[cfg(not(any(
    windows,
    target_os = "linux",
    target_os = "android",
    target_os = "macos",
    target_os = "ios"
)))]
pub fn resident_bytes() -> Option<u64> {
    None
}

#[cfg(all(test, any(windows, target_os = "linux", target_os = "macos")))]
mod tests {
    use super::*;

    /// The working set is the whole test process's, and one test caps it.
    static WORKING_SET: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn touched_memory_counts_as_resident() {
        let _alone = WORKING_SET.lock().unwrap();
        const SIZE: usize = 256 << 20;
        let before = resident_bytes().unwrap();
        let block = vec![1u8; SIZE];
        let during = resident_bytes().unwrap();
        std::hint::black_box(&block);
        assert!(
            during >= before + SIZE as u64 / 2,
            "{before} then {during} bytes"
        );
    }

    #[test]
    fn usage_says_how_tight_a_cap_nothing_enforces_is() {
        const GB: u64 = 1 << 30;
        assert_eq!(pressure_of_usage(GB / 2, GB), Pressure::Room);
        assert_eq!(pressure_of_usage(GB / 10 * 8, GB), Pressure::Steady);
        let Pressure::Over { keep } = pressure_of_usage(2 * GB, GB) else {
            panic!("twice the cap is over it");
        };
        assert!((keep - 0.4).abs() < 0.001, "{keep}");
    }

    #[test]
    fn every_system_steps_down_before_its_cap() {
        let _alone = WORKING_SET.lock().unwrap();
        let used = resident_bytes().unwrap();
        // Under a cap at half what it holds, the game is told to hold less, on Windows as
        // elsewhere: nothing pages it out to meet the cap instead.
        let Pressure::Over { keep } = pressure(used / 2) else {
            panic!("holding {used} bytes is over a cap of half that");
        };
        assert!(keep < 0.5, "{keep}");
        // Far below the cap it may hold more again.
        assert_eq!(pressure(used.saturating_mul(4)), Pressure::Room);
    }

    #[test]
    fn only_staying_over_with_nothing_left_to_give_up_ends_the_game() {
        let start = Instant::now();
        let later = |d: Duration| start + d;
        let mut over = Over::default();
        assert!(!over.gave_up(start, true, false));
        assert!(!over.gave_up(later(GRACE / 2), true, false));
        assert!(over.gave_up(later(GRACE), true, false));

        // Getting back under the cap starts the count again.
        let mut over = Over::default();
        assert!(!over.gave_up(start, true, false));
        assert!(!over.gave_up(later(GRACE / 2), false, false));
        assert!(!over.gave_up(later(GRACE), true, false));

        // So does having something left to give up, however long the game is over.
        let mut over = Over::default();
        assert!(!over.gave_up(start, true, false));
        assert!(!over.gave_up(later(GRACE * 5), true, true));
        assert!(!over.gave_up(later(GRACE * 6), true, false));

        // A cap turned off, then on again, or changed, gets its whole grace again.
        let mut over = Over::default();
        over.follow(Some(1 << 30));
        assert!(!over.gave_up(start, true, false));
        over.follow(None);
        over.follow(Some(1 << 30));
        assert!(!over.gave_up(later(GRACE), true, false));
        over.follow(Some(1 << 29));
        assert!(!over.gave_up(later(GRACE * 2), true, false));
        over.follow(Some(1 << 29));
        assert!(over.gave_up(later(GRACE * 3), true, false));
    }

    #[test]
    fn saying_the_game_can_shrink_holds_the_last_resort_off_for_a_while() {
        can_shrink();
        assert!(since_start() < SHRINKS_UNTIL.load(Ordering::Relaxed));
    }

    #[test]
    fn a_commit_budget_says_over_near_it_and_room_far_below() {
        assert_eq!(
            pressure_of_commit(950, 1000),
            Pressure::Over {
                keep: 0.8 * 1000. / 950.
            }
        );
        assert_eq!(pressure_of_commit(800, 1000), Pressure::Steady);
        assert_eq!(pressure_of_commit(600, 1000), Pressure::Room);
    }

    #[test]
    fn a_commit_budget_aims_for_less_the_further_over_it_is() {
        let a = pressure_of_commit(1000, 1000);
        let b = pressure_of_commit(2000, 1000);
        let (Pressure::Over { keep: a }, Pressure::Over { keep: b }) = (a, b) else {
            panic!("both are over");
        };
        assert!(a > b, "twice the budget asks for half as much");
        assert!((b - 0.4).abs() < 1e-9);
    }

    #[test]
    fn a_budget_of_zero_never_says_room() {
        assert!(matches!(pressure_of_commit(1, 0), Pressure::Over { .. }));
    }
}
