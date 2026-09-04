//! Which executables are running right now.
//!
//! Game profiles key off this. Matching on "is it running" rather than "is it
//! focused" is deliberate: a fullscreen game is the foreground window anyway,
//! and alt-tabbing out of one should not drop the machine back to a quiet
//! envelope while the match is still going.

use std::mem;

use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};

/// Snapshot of every running process name.
///
/// Enumerating costs a few milliseconds, so the caller should do this on a
/// slower cadence than the control loop rather than every tick.
pub fn running_names() -> Vec<String> {
    let mut names = Vec::new();

    let snap = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    if snap == INVALID_HANDLE_VALUE || snap.is_null() {
        return names;
    }

    let mut entry: PROCESSENTRY32W = unsafe { mem::zeroed() };
    entry.dwSize = mem::size_of::<PROCESSENTRY32W>() as u32;

    if unsafe { Process32FirstW(snap, &mut entry) } != 0 {
        loop {
            let end = entry
                .szExeFile
                .iter()
                .position(|&c| c == 0)
                .unwrap_or(entry.szExeFile.len());
            names.push(String::from_utf16_lossy(&entry.szExeFile[..end]));

            if unsafe { Process32NextW(snap, &mut entry) } == 0 {
                break;
            }
        }
    }

    unsafe { CloseHandle(snap) };
    names
}
