//! Physical memory hygiene.
//!
//! Windows keeps every page it has ever read in the standby list, so "used"
//! memory in Task Manager is mostly cache, not waste. That cache is handed back
//! the instant something asks for it, which is why blanket RAM cleaners are
//! snake oil: they throw away data that was costing nothing and force it to be
//! read from disk again.
//!
//! There is one real exception, and it is the only thing this module exists
//! for. The standby list is split by priority, and the bottom priorities hold
//! speculatively-read pages that nothing has touched twice. When a game
//! allocates several gigabytes in a few seconds, the memory manager has to
//! repurpose standby pages to satisfy it, and that repurposing happens at
//! fault time - which is a stutter. Emptying the low-priority end *before* the
//! allocation burst turns that work into free pages ahead of time.
//!
//! So the rule here is: reclaim at the moments it buys something (a game
//! starting, a game exiting, genuine free memory running out) and never on a
//! timer. The useful cache is left alone unless explicitly asked otherwise.

use std::ffi::c_void;
use std::time::Instant;

use crate::config::MemoryCfg;

// ---------------------------------------------------------------------------
// Win32 / NT bindings
//
// The memory-list calls are undocumented NT internals with no windows-sys
// binding, and the token calls come out cleaner declared alongside them than
// split across two type systems. Handles are plain `isize`, which is
// ABI-identical to HANDLE.
// ---------------------------------------------------------------------------

const SYSTEM_MEMORY_LIST_INFORMATION: u32 = 80;

// SYSTEM_MEMORY_LIST_COMMAND
const MEMORY_EMPTY_WORKING_SETS: i32 = 2;
const MEMORY_FLUSH_MODIFIED_LIST: i32 = 3;
const MEMORY_PURGE_STANDBY_LIST: i32 = 4;
const MEMORY_PURGE_LOW_PRIORITY_STANDBY_LIST: i32 = 5;

/// x64 Windows expresses every one of these page counts in 4 KB pages.
const PAGE_BYTES: u64 = 4096;

const TOKEN_ADJUST_PRIVILEGES: u32 = 0x0020;
const TOKEN_QUERY: u32 = 0x0008;
const SE_PRIVILEGE_ENABLED: u32 = 0x0002;
const ERROR_NOT_ALL_ASSIGNED: u32 = 1300;

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct SystemMemoryListInformation {
    zero_page_count: usize,
    free_page_count: usize,
    modified_page_count: usize,
    modified_no_write_page_count: usize,
    bad_page_count: usize,
    page_count_by_priority: [usize; 8],
    repurposed_pages_by_priority: [usize; 8],
    modified_page_count_page_file: usize,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct MemoryStatusEx {
    length: u32,
    memory_load: u32,
    total_phys: u64,
    avail_phys: u64,
    total_page_file: u64,
    avail_page_file: u64,
    total_virtual: u64,
    avail_virtual: u64,
    avail_extended_virtual: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct Luid {
    low: u32,
    high: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct LuidAndAttributes {
    luid: Luid,
    attributes: u32,
}

#[repr(C)]
struct TokenPrivileges {
    count: u32,
    privileges: [LuidAndAttributes; 1],
}

#[link(name = "kernel32")]
extern "system" {
    fn GlobalMemoryStatusEx(buffer: *mut MemoryStatusEx) -> i32;
    fn SetSystemFileCacheSize(min: usize, max: usize, flags: u32) -> i32;
    fn GetCurrentProcess() -> isize;
    fn CloseHandle(h: isize) -> i32;
    fn GetLastError() -> u32;
}

#[link(name = "advapi32")]
extern "system" {
    fn OpenProcessToken(process: isize, access: u32, token: *mut isize) -> i32;
    fn LookupPrivilegeValueW(system: *const u16, name: *const u16, luid: *mut Luid) -> i32;
    fn AdjustTokenPrivileges(
        token: isize,
        disable_all: i32,
        new_state: *const TokenPrivileges,
        len: u32,
        previous: *mut c_void,
        ret_len: *mut u32,
    ) -> i32;
}

type NtQuerySystemInformationFn =
    unsafe extern "system" fn(u32, *mut c_void, u32, *mut u32) -> i32;
type NtSetSystemInformationFn = unsafe extern "system" fn(u32, *mut c_void, u32) -> i32;

/// Resolve the two ntdll entry points once. ntdll is mapped into every process,
/// so this cannot fail for any reason worth retrying.
struct Ntdll {
    query: NtQuerySystemInformationFn,
    set: NtSetSystemInformationFn,
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

impl Ntdll {
    fn resolve() -> Option<Self> {
        use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};

        unsafe {
            let h = GetModuleHandleW(wide("ntdll.dll").as_ptr());
            if h.is_null() {
                return None;
            }
            let q = GetProcAddress(h, c"NtQuerySystemInformation".as_ptr() as *const u8)?;
            let s = GetProcAddress(h, c"NtSetSystemInformation".as_ptr() as *const u8)?;
            Some(Self {
                query: std::mem::transmute::<_, NtQuerySystemInformationFn>(q),
                set: std::mem::transmute::<_, NtSetSystemInformationFn>(s),
            })
        }
    }
}

/// Turn on a named privilege for this process.
///
/// Purging the standby list needs `SeProfileSingleProcessPrivilege` and
/// trimming the file cache needs `SeIncreaseQuotaPrivilege`. Both are held by
/// an elevated token but are *disabled* by default, so having them is not the
/// same as being able to use them.
fn enable_privilege(name: &str) -> bool {
    unsafe {
        let mut token: isize = 0;
        if OpenProcessToken(
            GetCurrentProcess(),
            TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY,
            &mut token,
        ) == 0
        {
            return false;
        }

        let mut luid = Luid::default();
        let ok = if LookupPrivilegeValueW(std::ptr::null(), wide(name).as_ptr(), &mut luid) == 0 {
            false
        } else {
            let tp = TokenPrivileges {
                count: 1,
                privileges: [LuidAndAttributes {
                    luid,
                    attributes: SE_PRIVILEGE_ENABLED,
                }],
            };
            let called = AdjustTokenPrivileges(
                token,
                0,
                &tp,
                std::mem::size_of::<TokenPrivileges>() as u32,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            );
            // AdjustTokenPrivileges reports success even when it assigned
            // nothing, so the error code is the only honest answer.
            called != 0 && GetLastError() != ERROR_NOT_ALL_ASSIGNED
        };

        CloseHandle(token);
        ok
    }
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

/// A snapshot of where physical memory actually is.
#[derive(Debug, Clone, Copy, Default)]
pub struct MemState {
    pub total_mb: u64,
    /// What Windows advertises as available. Includes the whole standby list,
    /// which is why it looks healthy even when free memory is nearly gone.
    pub avail_mb: u64,
    /// Zero + free lists: pages that can be handed out with no work at all.
    pub free_mb: u64,
    /// Dirty pages awaiting a disk write.
    pub modified_mb: u64,
    /// Every standby priority summed.
    pub standby_mb: u64,
    /// Per-priority standby, in MB, lowest first.
    ///
    /// Kept raw rather than pre-bucketed because the kernel decides what
    /// "low priority" means when purging, and the only way to know which
    /// priorities a purge actually takes is to look at these before and after.
    pub standby_by_priority_mb: [u64; 8],
    /// Priorities 0-4: speculatively read, touched once, unlikely to be wanted
    /// again.
    ///
    /// The split is not a guess. Read side by side with Windows' own counters
    /// on this machine, `Standby Cache Reserve` came to 6345 MB against 6342 MB
    /// for priorities 0-4, `Standby Cache Normal Priority` to 8612 MB against
    /// 8613 MB for priority 5 alone, and `Standby Cache Core` to 61 MB against
    /// 60 MB for priorities 6-7. So the kernel's own idea of "reserve" is
    /// everything below priority 5, and that is what we target.
    pub standby_low_mb: u64,
    /// Priority 5: ordinary file cache, the part actually earning its keep.
    pub standby_normal_mb: u64,
    /// Priorities 6-7: hot, repeatedly used cache. Never targeted.
    pub standby_core_mb: u64,
    pub load_pct: u32,
}

fn mb(pages: usize) -> u64 {
    (pages as u64 * PAGE_BYTES) / (1024 * 1024)
}

impl MemState {
    pub fn read() -> Option<Self> {
        let nt = Ntdll::resolve()?;

        let mut info = SystemMemoryListInformation::default();
        let mut ret: u32 = 0;
        let st = unsafe {
            (nt.query)(
                SYSTEM_MEMORY_LIST_INFORMATION,
                &mut info as *mut _ as *mut c_void,
                std::mem::size_of::<SystemMemoryListInformation>() as u32,
                &mut ret,
            )
        };
        if st != 0 {
            return None;
        }

        let mut status = MemoryStatusEx {
            length: std::mem::size_of::<MemoryStatusEx>() as u32,
            ..Default::default()
        };
        if unsafe { GlobalMemoryStatusEx(&mut status) } == 0 {
            return None;
        }

        let p = &info.page_count_by_priority;
        let mut by_priority = [0u64; 8];
        for (i, slot) in by_priority.iter_mut().enumerate() {
            *slot = mb(p[i]);
        }

        Some(Self {
            total_mb: status.total_phys / (1024 * 1024),
            avail_mb: status.avail_phys / (1024 * 1024),
            free_mb: mb(info.zero_page_count + info.free_page_count),
            modified_mb: mb(info.modified_page_count + info.modified_no_write_page_count),
            standby_mb: mb(p.iter().sum()),
            standby_by_priority_mb: by_priority,
            standby_low_mb: mb(p[0] + p[1] + p[2] + p[3] + p[4]),
            standby_normal_mb: mb(p[5]),
            standby_core_mb: mb(p[6] + p[7]),
            load_pct: status.memory_load,
        })
    }
}

// ---------------------------------------------------------------------------
// Actions
// ---------------------------------------------------------------------------

fn command(nt: &Ntdll, cmd: i32) -> bool {
    let mut c = cmd;
    unsafe { (nt.set)(SYSTEM_MEMORY_LIST_INFORMATION, &mut c as *mut _ as *mut c_void, 4) == 0 }
}

/// What a reclaim pass is allowed to touch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Depth {
    /// Priorities 0-1 only. Costs nothing worth having.
    Low,
    /// The entire standby list plus the system file cache. Useful cache is lost
    /// and will be re-read from disk.
    Full,
    /// Everything above, plus trimming every process working set. Visibly
    /// stutters the desktop as applications fault their pages back in; only
    /// worth it when memory is genuinely exhausted.
    Deep,
}

impl Depth {
    pub fn label(self) -> &'static str {
        match self {
            Depth::Low => "dusuk oncelikli",
            Depth::Full => "tam standby",
            Depth::Deep => "derin",
        }
    }
}

/// Result of one reclaim pass.
#[derive(Debug, Clone, Copy)]
pub struct Reclaimed {
    pub before: MemState,
    pub after: MemState,
    pub depth: Depth,
}

impl Reclaimed {
    /// How much moved into the genuinely-free pool.
    pub fn freed_mb(&self) -> u64 {
        self.after.free_mb.saturating_sub(self.before.free_mb)
    }

    pub fn line(&self) -> String {
        format!(
            "RAM geri kazanildi ({}): bos {} -> {} MB (+{} MB), standby {} -> {} MB",
            self.depth.label(),
            self.before.free_mb,
            self.after.free_mb,
            self.freed_mb(),
            self.before.standby_mb,
            self.after.standby_mb,
        )
    }
}

/// Run one reclaim pass at the given depth.
pub fn reclaim(depth: Depth) -> Option<Reclaimed> {
    let nt = Ntdll::resolve()?;
    let before = MemState::read()?;

    // Enabled per pass rather than once at startup: the privilege costs nothing
    // to re-assert, and doing it here means a daemon that was started
    // unprivileged and later re-launched elevated starts working immediately.
    enable_privilege("SeProfileSingleProcessPrivilege");

    command(&nt, MEMORY_PURGE_LOW_PRIORITY_STANDBY_LIST);

    if depth != Depth::Low {
        command(&nt, MEMORY_FLUSH_MODIFIED_LIST);
        command(&nt, MEMORY_PURGE_STANDBY_LIST);
        if enable_privilege("SeIncreaseQuotaPrivilege") {
            // (-1, -1) means "reset to the default working set range", which
            // drops what the file cache is currently holding resident.
            unsafe { SetSystemFileCacheSize(usize::MAX, usize::MAX, 0) };
        }
    }

    if depth == Depth::Deep {
        command(&nt, MEMORY_EMPTY_WORKING_SETS);
    }

    // The lists settle asynchronously; sampling immediately understates it.
    std::thread::sleep(std::time::Duration::from_millis(300));
    let after = MemState::read()?;

    Some(Reclaimed { before, after, depth })
}

// ---------------------------------------------------------------------------
// Policy
// ---------------------------------------------------------------------------

/// Why a pass fired. Kept for the log line, because "it reclaimed 6 GB" is only
/// reassuring if you can also see it was not doing that every minute.
#[derive(Debug, Clone, Copy)]
pub enum Trigger {
    /// A heavy workload just started or just ended - the allocation burst is
    /// either imminent or has just left its residue behind.
    ModeChange,
    /// Genuinely free pages ran low while the standby list was large.
    Pressure,
    /// Free memory is nearly gone, whatever the standby list is made of.
    Critical,
}

impl Trigger {
    fn label(self) -> &'static str {
        match self {
            Trigger::ModeChange => "mod degisimi",
            Trigger::Pressure => "bellek baskisi",
            Trigger::Critical => "BELLEK KRITIK",
        }
    }
}

/// A pass that returns nothing means the condition it was meant to relieve is
/// not one this depth can relieve. Retrying on the same schedule just burns
/// cycles, so the wait grows - up to this many multiples of the configured
/// cooldown - until something actually changes.
const MAX_BACKOFF: u32 = 8;

/// Below this yield, a pass counts as having achieved nothing.
const FRUITLESS_MB: u64 = 32;

pub struct Reclaimer {
    cfg: MemoryCfg,
    last: Option<Instant>,
    /// Cooldown multiplier, grown after a pass that freed nothing.
    backoff: u32,
    pub passes: u32,
    pub total_freed_mb: u64,
    pub state: MemState,
}

impl Reclaimer {
    pub fn new(cfg: MemoryCfg) -> Self {
        Self {
            cfg,
            last: None,
            backoff: 1,
            passes: 0,
            total_freed_mb: 0,
            state: MemState::read().unwrap_or_default(),
        }
    }

    /// Refresh the cached snapshot. Cheap enough to call on the slow poll.
    pub fn refresh(&mut self) {
        if let Some(s) = MemState::read() {
            self.state = s;
        }
    }

    fn cooled_down(&self) -> bool {
        match self.last {
            None => true,
            Some(t) => t.elapsed().as_secs() >= self.cfg.cooldown_s * self.backoff as u64,
        }
    }

    /// Decide whether to reclaim, and do it. Returns a line to log when it acted.
    ///
    /// `mode_shifted` is true on the tick where the workload crossed into or out
    /// of a heavy mode.
    pub fn tick(&mut self, mode_shifted: bool) -> Option<String> {
        if !self.cfg.enabled {
            return None;
        }
        self.refresh();
        let s = self.state;

        // Nothing to take. Acting anyway would only discard useful cache.
        if s.standby_low_mb < self.cfg.min_standby_mb && s.standby_mb < self.cfg.min_standby_mb {
            return None;
        }

        let (trigger, depth) = if mode_shifted && self.cfg.on_mode_change {
            // A discrete, rare event - not subject to the cooldown, or the
            // launch it exists to smooth would be the one pass it skips.
            //
            // If memory is *already* short as the game starts, do the expensive
            // pass here rather than let the mid-session trigger catch it later.
            // Both cost the same to run; the difference is when the re-reads
            // land. Here it is a loading screen, which absorbs it invisibly.
            // Thirty seconds later it is a rendered frame, and the player feels
            // it as a stutter - which is exactly what was happening.
            let depth = if s.free_mb < self.cfg.critical_free_mb {
                Depth::Full
            } else {
                Depth::Low
            };
            (Trigger::ModeChange, depth)
        } else if s.free_mb < self.cfg.critical_free_mb && self.cooled_down() {
            // Starving. At this point the safe pass has usually already been
            // tried and found the throwaway end of the list empty, so holding
            // back the rest of the cache is protecting the wrong thing.
            (Trigger::Critical, Depth::Full)
        } else if s.free_mb < self.cfg.min_free_mb && self.cooled_down() {
            // Real pressure: free pages are nearly gone and the memory manager
            // is about to start repurposing standby at fault time.
            let depth = if self.cfg.aggressive { Depth::Full } else { Depth::Low };
            (Trigger::Pressure, depth)
        } else {
            return None;
        };

        self.run(trigger, depth)
    }

    fn run(&mut self, trigger: Trigger, depth: Depth) -> Option<String> {
        let r = reclaim(depth)?;
        self.last = Some(Instant::now());
        self.passes += 1;
        self.total_freed_mb += r.freed_mb();
        self.state = r.after;

        // Back off when a pass achieves nothing, and snap straight back to the
        // normal interval as soon as one works. Without this the daemon spent
        // hours repeating a purge that had nothing left to take.
        if r.freed_mb() < FRUITLESS_MB {
            self.backoff = (self.backoff * 2).min(MAX_BACKOFF);
            // Nothing was achieved, so nothing is worth reporting.
            return None;
        }
        self.backoff = 1;
        Some(format!("[{}] {}", trigger.label(), r.line()))
    }
}
