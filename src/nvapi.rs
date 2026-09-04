//! The NVIDIA driver settings repository (DRS), over NVAPI.
//!
//! This is the database behind the NVIDIA Control Panel: frame rate limiter,
//! power management mode, texture filtering quality and the rest. NVML cannot
//! reach any of it, and on this machine NVML's power limit is refused by the
//! vBIOS anyway, so DRS is where the remaining GPU levers actually live.
//!
//! Two things make this safe to do from code rather than reckless.
//!
//! First, nothing is written against a guessed setting ID. The driver can list
//! every setting it supports and hand back each one's name, so every ID used
//! here is confirmed against that list at runtime; an ID that does not resolve
//! to the expected name is skipped rather than written.
//!
//! Second, the struct NVAPI expects is version-stamped with its own size. Get
//! that wrong and the call fails with a status code instead of scribbling, and
//! the buffer used here is far larger than the struct can plausibly be, so a
//! layout mistake cannot run off the end of it.
#![allow(clippy::missing_transmute_annotations, clippy::type_complexity)]

use std::ffi::c_void;

use anyhow::{anyhow, Result};

type Status = i32;
const NVAPI_OK: Status = 0;
const NVAPI_INCOMPATIBLE_STRUCT_VERSION: Status = -9;

/// Opaque NVAPI handles.
type SessionHandle = isize;
type ProfileHandle = isize;

// Function ids for nvapi_QueryInterface. These are stable across driver
// versions; a wrong one simply fails to resolve and is reported.
const ID_INITIALIZE: u32 = 0x0150E828;
const ID_UNLOAD: u32 = 0xD22BDD7E;
const ID_DRS_CREATE_SESSION: u32 = 0x0694D52E;
const ID_DRS_DESTROY_SESSION: u32 = 0xDAD9CFF8;
const ID_DRS_LOAD_SETTINGS: u32 = 0x375DBD6B;
const ID_DRS_SAVE_SETTINGS: u32 = 0xFCBC7E14;
const ID_DRS_GET_BASE_PROFILE: u32 = 0xDA8466A0;
const ID_DRS_GET_SETTING: u32 = 0x73BF8338;
const ID_DRS_SET_SETTING: u32 = 0x577DD202;
const ID_DRS_ENUM_AVAILABLE_SETTING_IDS: u32 = 0xF020614A;
const ID_DRS_GET_SETTING_NAME_FROM_ID: u32 = 0xD61CBE6E;
const ID_DRS_RESTORE_PROFILE_DEFAULT_SETTING: u32 = 0x53DABBCA;
/// Removes a setting from a profile outright, rather than setting it to a
/// default value. This is the true inverse of adding one: a setting rogctl
/// introduced was not present beforehand, so the driver has no default to
/// restore and only deletion puts the profile back as it was.
const ID_DRS_DELETE_PROFILE_SETTING: u32 = 0xE4A26362;
/// Walking every profile, not just the base one.
///
/// The base profile is the global default and it is the *weakest* thing in the
/// repository: any application profile that names the same setting overrides it
/// for that executable. Reading only the base profile therefore answers "what
/// did rogctl ask for", never "what will the game actually get" - and when
/// those two disagree, the second one is the one on screen.
const ID_DRS_ENUM_PROFILES: u32 = 0xBC371EE0;
const ID_DRS_GET_PROFILE_INFO: u32 = 0x61CD6FD6;

/// NvAPI_UnicodeString is a fixed 2048-wide-character array.
const UNICODE_MAX: usize = 2048;

// NVDRS_SETTING layout, in bytes from the start of the struct. Derived from the
// published header and confirmed at runtime by `probe_version` below: if these
// offsets were wrong the size would be wrong too, and the driver rejects a
// wrong size outright rather than accepting it.
const OFF_VERSION: usize = 0;
const OFF_SETTING_NAME: usize = 4;
const OFF_SETTING_ID: usize = OFF_SETTING_NAME + UNICODE_MAX * 2; // 4100
const OFF_SETTING_TYPE: usize = OFF_SETTING_ID + 4; // 4104
const OFF_SETTING_LOCATION: usize = OFF_SETTING_TYPE + 4; // 4108
const OFF_IS_CURRENT_PREDEFINED: usize = OFF_SETTING_LOCATION + 4; // 4112
const OFF_IS_PREDEFINED_VALID: usize = OFF_IS_CURRENT_PREDEFINED + 4; // 4116
/// Each value union is a NVDRS_BINARY_SETTING at its largest: u32 length plus
/// a 4096-byte payload.
const UNION_SIZE: usize = 4 + 4096;
const OFF_PREDEFINED_VALUE: usize = OFF_IS_PREDEFINED_VALID + 4; // 4120
const OFF_CURRENT_VALUE: usize = OFF_PREDEFINED_VALUE + UNION_SIZE; // 8220
const SETTING_SIZE: usize = OFF_CURRENT_VALUE + UNION_SIZE; // 12320

/// Room to spare, so a layout error cannot become a buffer overrun.
const SETTING_BUF: usize = 20480;

// NVDRS_PROFILE layout, same derivation and same runtime confirmation as
// NVDRS_SETTING above.
const OFF_PROFILE_NAME: usize = 4;
const OFF_PROFILE_GPU_SUPPORT: usize = OFF_PROFILE_NAME + UNICODE_MAX * 2; // 4100
const OFF_PROFILE_IS_PREDEFINED: usize = OFF_PROFILE_GPU_SUPPORT + 4; // 4104
const OFF_PROFILE_NUM_APPS: usize = OFF_PROFILE_IS_PREDEFINED + 4; // 4108
const OFF_PROFILE_NUM_SETTINGS: usize = OFF_PROFILE_NUM_APPS + 4; // 4112
const PROFILE_SIZE: usize = OFF_PROFILE_NUM_SETTINGS + 4; // 4116

/// NVDRS_SETTING_TYPE::NVDRS_DWORD_TYPE
const TYPE_DWORD: u32 = 0;

/// NVDRS_SETTING_LOCATION::NVDRS_CURRENT_PROFILE_LOCATION - the value is stored
/// in the profile being read, rather than inherited from the global one.
const LOCATION_CURRENT_PROFILE: u32 = 0;

fn make_version(size: usize, ver: u32) -> u32 {
    (size as u32) | (ver << 16)
}

type FnInitialize = unsafe extern "C" fn() -> Status;
type FnUnload = unsafe extern "C" fn() -> Status;
type FnCreateSession = unsafe extern "C" fn(*mut SessionHandle) -> Status;
type FnSessionOnly = unsafe extern "C" fn(SessionHandle) -> Status;
type FnGetBaseProfile = unsafe extern "C" fn(SessionHandle, *mut ProfileHandle) -> Status;
type FnGetSetting = unsafe extern "C" fn(SessionHandle, ProfileHandle, u32, *mut u8) -> Status;
type FnSetSetting = unsafe extern "C" fn(SessionHandle, ProfileHandle, *mut u8) -> Status;
type FnEnumIds = unsafe extern "C" fn(*mut u32, *mut u32) -> Status;
type FnNameFromId = unsafe extern "C" fn(u32, *mut u16) -> Status;
type FnRestoreDefault = unsafe extern "C" fn(SessionHandle, ProfileHandle, u32) -> Status;
type FnEnumProfiles = unsafe extern "C" fn(SessionHandle, u32, *mut ProfileHandle) -> Status;
type FnGetProfileInfo = unsafe extern "C" fn(SessionHandle, ProfileHandle, *mut u8) -> Status;

type FnQueryInterface = unsafe extern "C" fn(u32) -> *mut c_void;

pub struct NvApi {
    unload: FnUnload,
    create_session: FnCreateSession,
    destroy_session: FnSessionOnly,
    load_settings: FnSessionOnly,
    save_settings: FnSessionOnly,
    get_base_profile: FnGetBaseProfile,
    get_setting: FnGetSetting,
    set_setting: FnSetSetting,
    enum_ids: FnEnumIds,
    name_from_id: FnNameFromId,
    restore_default: FnRestoreDefault,
    delete_setting: FnRestoreDefault,
    enum_profiles: FnEnumProfiles,
    get_profile_info: FnGetProfileInfo,
    /// Version stamp the driver accepts for NVDRS_SETTING, established by probe.
    setting_version: u32,
    /// Same, for NVDRS_PROFILE.
    profile_version: u32,
}

macro_rules! resolve {
    ($q:expr, $id:expr, $t:ty, $what:literal) => {{
        let p = unsafe { $q($id) };
        if p.is_null() {
            return Err(anyhow!(concat!("NVAPI ", $what, " cozumlenemedi")));
        }
        unsafe { std::mem::transmute::<*mut c_void, $t>(p) }
    }};
}

fn wide_to_string(buf: &[u16]) -> String {
    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..end])
}

impl NvApi {
    pub fn open() -> Result<Self> {
        use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};

        let name: Vec<u16> = "nvapi64.dll".encode_utf16().chain(std::iter::once(0)).collect();
        let lib = unsafe { LoadLibraryW(name.as_ptr()) };
        if lib.is_null() {
            return Err(anyhow!("nvapi64.dll yuklenemedi"));
        }
        let q = unsafe { GetProcAddress(lib, c"nvapi_QueryInterface".as_ptr() as *const u8) }
            .ok_or_else(|| anyhow!("nvapi_QueryInterface bulunamadi"))?;
        let query: FnQueryInterface = unsafe { std::mem::transmute(q) };

        let init = resolve!(query, ID_INITIALIZE, FnInitialize, "Initialize");
        let st = unsafe { init() };
        if st != NVAPI_OK {
            return Err(anyhow!("NvAPI_Initialize basarisiz (durum {st})"));
        }

        let mut me = Self {
            unload: resolve!(query, ID_UNLOAD, FnUnload, "Unload"),
            create_session: resolve!(query, ID_DRS_CREATE_SESSION, FnCreateSession, "CreateSession"),
            destroy_session: resolve!(query, ID_DRS_DESTROY_SESSION, FnSessionOnly, "DestroySession"),
            load_settings: resolve!(query, ID_DRS_LOAD_SETTINGS, FnSessionOnly, "LoadSettings"),
            save_settings: resolve!(query, ID_DRS_SAVE_SETTINGS, FnSessionOnly, "SaveSettings"),
            get_base_profile: resolve!(query, ID_DRS_GET_BASE_PROFILE, FnGetBaseProfile, "GetBaseProfile"),
            get_setting: resolve!(query, ID_DRS_GET_SETTING, FnGetSetting, "GetSetting"),
            set_setting: resolve!(query, ID_DRS_SET_SETTING, FnSetSetting, "SetSetting"),
            enum_ids: resolve!(query, ID_DRS_ENUM_AVAILABLE_SETTING_IDS, FnEnumIds, "EnumAvailableSettingIds"),
            name_from_id: resolve!(query, ID_DRS_GET_SETTING_NAME_FROM_ID, FnNameFromId, "GetSettingNameFromId"),
            restore_default: resolve!(query, ID_DRS_RESTORE_PROFILE_DEFAULT_SETTING, FnRestoreDefault, "RestoreProfileDefaultSetting"),
            delete_setting: resolve!(query, ID_DRS_DELETE_PROFILE_SETTING, FnRestoreDefault, "DeleteProfileSetting"),
            enum_profiles: resolve!(query, ID_DRS_ENUM_PROFILES, FnEnumProfiles, "EnumProfiles"),
            get_profile_info: resolve!(query, ID_DRS_GET_PROFILE_INFO, FnGetProfileInfo, "GetProfileInfo"),
            setting_version: make_version(SETTING_SIZE, 1),
            profile_version: make_version(PROFILE_SIZE, 1),
        };

        me.probe_version()?;
        Ok(me)
    }

    /// Confirm the struct version the driver will accept.
    ///
    /// The size is part of the version word, so a layout that is even four
    /// bytes off is rejected. Rather than trust the derivation, read a setting
    /// that is known to exist and step through candidate sizes until one is not
    /// refused - then that is the layout the installed driver is using.
    fn probe_version(&mut self) -> Result<()> {
        let ids = self.available_setting_ids();
        let probe_id = *ids.first().ok_or_else(|| anyhow!("surucu hic ayar bildirmedi"))?;

        let session = self.session()?;
        let mut last = 0;
        for delta in [0isize, -4, 4, -8, 8, -4100, 4100] {
            let size = (SETTING_SIZE as isize + delta) as usize;
            let ver = make_version(size, 1);
            let mut buf = vec![0u8; SETTING_BUF];
            buf[OFF_VERSION..OFF_VERSION + 4].copy_from_slice(&ver.to_le_bytes());

            let st = unsafe { (self.get_setting)(session.0, session.1, probe_id, buf.as_mut_ptr()) };
            last = st;
            if st != NVAPI_INCOMPATIBLE_STRUCT_VERSION {
                self.setting_version = ver;
                return Ok(());
            }
        }
        Err(anyhow!(
            "NVDRS_SETTING yapisinin surumu bulunamadi (son durum {last}) - bu surucude DRS yazilamaz"
        ))
    }

    /// Every setting id the installed driver knows about.
    pub fn available_setting_ids(&self) -> Vec<u32> {
        let mut ids = vec![0u32; 4096];
        let mut count = ids.len() as u32;
        let st = unsafe { (self.enum_ids)(ids.as_mut_ptr(), &mut count) };
        if st != NVAPI_OK {
            return Vec::new();
        }
        ids.truncate(count as usize);
        ids
    }

    /// The driver's own name for a setting id. This is what makes writing safe:
    /// an id is only used once its name has confirmed what it is.
    pub fn setting_name(&self, id: u32) -> Option<String> {
        let mut buf = vec![0u16; UNICODE_MAX];
        let st = unsafe { (self.name_from_id)(id, buf.as_mut_ptr()) };
        if st != NVAPI_OK {
            return None;
        }
        let s = wide_to_string(&buf);
        if s.is_empty() {
            None
        } else {
            Some(s)
        }
    }

    fn session(&self) -> Result<(SessionHandle, ProfileHandle)> {
        let mut s: SessionHandle = 0;
        let st = unsafe { (self.create_session)(&mut s) };
        if st != NVAPI_OK {
            return Err(anyhow!("DRS oturumu acilamadi (durum {st})"));
        }
        let st = unsafe { (self.load_settings)(s) };
        if st != NVAPI_OK {
            unsafe { (self.destroy_session)(s) };
            return Err(anyhow!("DRS ayarlari yuklenemedi (durum {st})"));
        }
        let mut p: ProfileHandle = 0;
        let st = unsafe { (self.get_base_profile)(s, &mut p) };
        if st != NVAPI_OK {
            unsafe { (self.destroy_session)(s) };
            return Err(anyhow!("DRS temel profili alinamadi (durum {st})"));
        }
        Ok((s, p))
    }

    /// Read one DWORD setting from the global profile.
    ///
    /// Returns the value and whether it is still the driver's own default.
    pub fn get_u32(&self, id: u32) -> Option<(u32, bool)> {
        let (s, p) = self.session().ok()?;
        let out = self.get_u32_in(s, p, id);
        unsafe { (self.destroy_session)(s) };
        out
    }

    fn get_u32_in(&self, s: SessionHandle, p: ProfileHandle, id: u32) -> Option<(u32, bool)> {
        let (val, predefined, _) = self.get_setting_in(s, p, id)?;
        Some((val, predefined))
    }

    /// As `get_u32_in`, but also reporting *where* the value came from.
    ///
    /// A profile that does not carry a setting still answers a read of it: the
    /// driver walks up to the global profile and returns that value instead.
    /// Without `settingLocation` there is no way to tell "this game is pinned to
    /// 60" from "this game inherits the global 141", and every profile in the
    /// repository looks like it has an opinion when almost none of them do.
    fn get_setting_in(
        &self,
        s: SessionHandle,
        p: ProfileHandle,
        id: u32,
    ) -> Option<(u32, bool, u32)> {
        let mut buf = vec![0u8; SETTING_BUF];
        buf[OFF_VERSION..OFF_VERSION + 4].copy_from_slice(&self.setting_version.to_le_bytes());
        if unsafe { (self.get_setting)(s, p, id, buf.as_mut_ptr()) } != NVAPI_OK {
            return None;
        }
        let val = u32::from_le_bytes(buf[OFF_CURRENT_VALUE..OFF_CURRENT_VALUE + 4].try_into().ok()?);
        let predefined = u32::from_le_bytes(
            buf[OFF_IS_CURRENT_PREDEFINED..OFF_IS_CURRENT_PREDEFINED + 4].try_into().ok()?,
        );
        let location = u32::from_le_bytes(
            buf[OFF_SETTING_LOCATION..OFF_SETTING_LOCATION + 4].try_into().ok()?,
        );
        Some((val, predefined != 0, location))
    }

    /// The value the driver itself declares as this setting's default, if it
    /// declares one.
    ///
    /// `NVDRS_SETTING` carries the predefined value alongside the current one,
    /// so the stock value can be read rather than assumed. This is what makes
    /// "put it back" possible without knowing what the default happens to be.
    fn predefined_u32(&self, s: SessionHandle, p: ProfileHandle, id: u32) -> Option<u32> {
        let mut buf = vec![0u8; SETTING_BUF];
        buf[OFF_VERSION..OFF_VERSION + 4].copy_from_slice(&self.setting_version.to_le_bytes());
        if unsafe { (self.get_setting)(s, p, id, buf.as_mut_ptr()) } != NVAPI_OK {
            return None;
        }
        let valid = u32::from_le_bytes(
            buf[OFF_IS_PREDEFINED_VALID..OFF_IS_PREDEFINED_VALID + 4].try_into().ok()?,
        );
        if valid == 0 {
            return None;
        }
        Some(u32::from_le_bytes(
            buf[OFF_PREDEFINED_VALUE..OFF_PREDEFINED_VALUE + 4].try_into().ok()?,
        ))
    }

    /// The name of a profile, plus whether it shipped with the driver.
    fn profile_info(&self, s: SessionHandle, p: ProfileHandle) -> Option<(String, bool)> {
        for ver in [self.profile_version, make_version(PROFILE_SIZE, 2)] {
            let mut buf = vec![0u8; SETTING_BUF];
            buf[OFF_VERSION..OFF_VERSION + 4].copy_from_slice(&ver.to_le_bytes());
            if unsafe { (self.get_profile_info)(s, p, buf.as_mut_ptr()) } != NVAPI_OK {
                continue;
            }
            let name: Vec<u16> = buf[OFF_PROFILE_NAME..OFF_PROFILE_NAME + UNICODE_MAX * 2]
                .chunks_exact(2)
                .map(|c| u16::from_le_bytes([c[0], c[1]]))
                .collect();
            let predefined = u32::from_le_bytes(
                buf[OFF_PROFILE_IS_PREDEFINED..OFF_PROFILE_IS_PREDEFINED + 4]
                    .try_into()
                    .ok()?,
            );
            return Some((wide_to_string(&name), predefined != 0));
        }
        None
    }

    /// Every profile in the repository that carries `id`, with its value.
    ///
    /// This is the question the base profile cannot answer. rogctl writes the
    /// frame cap globally, but an application profile naming the same setting
    /// wins for that executable - so when the number on screen disagrees with
    /// the number rogctl wrote, this is where the disagreement is visible.
    pub fn setting_across_profiles(&self, id: u32) -> Result<Vec<ProfileValue>> {
        let (s, _) = self.session()?;
        let mut out = Vec::new();
        let mut i = 0u32;
        loop {
            let mut p: ProfileHandle = 0;
            if unsafe { (self.enum_profiles)(s, i, &mut p) } != NVAPI_OK {
                break;
            }
            i += 1;
            // Only values the profile actually carries. Anything else is the
            // global setting being read back through this profile.
            if let Some((value, predefined, LOCATION_CURRENT_PROFILE)) =
                self.get_setting_in(s, p, id)
            {
                let (profile, driver_supplied) = self
                    .profile_info(s, p)
                    .unwrap_or_else(|| (format!("<profil #{i}>"), false));
                out.push(ProfileValue {
                    profile,
                    driver_supplied,
                    value,
                    predefined,
                });
            }
            // A repository this size is thousands of profiles at most; the
            // bound is here so a driver that never fails the enum cannot spin.
            if i > 100_000 {
                break;
            }
        }
        unsafe { (self.destroy_session)(s) };
        Ok(out)
    }

    /// Every setting a specific profile carries itself, name-matched rather than
    /// id-matched.
    ///
    /// `setting_across_profiles` answers "who has this one setting" for a
    /// setting already suspected. This answers the wider question for a single
    /// profile: not "is it the frame limiter" but "what does this profile touch
    /// at all" - for when the suspect list turns out to be the wrong list.
    pub fn profile_settings(&self, name_filter: &str) -> Result<Vec<(String, String, u32, bool)>> {
        let (s, _) = self.session()?;
        let wanted = name_filter.to_uppercase();
        let all_ids = self.available_setting_ids();
        let mut out = Vec::new();

        let mut i = 0u32;
        loop {
            let mut p: ProfileHandle = 0;
            if unsafe { (self.enum_profiles)(s, i, &mut p) } != NVAPI_OK {
                break;
            }
            i += 1;
            let Some((profile_name, _)) = self.profile_info(s, p) else { continue };
            if !profile_name.to_uppercase().contains(&wanted) {
                continue;
            }
            for &id in &all_ids {
                if let Some((value, predefined, LOCATION_CURRENT_PROFILE)) =
                    self.get_setting_in(s, p, id)
                {
                    let setting_name = self.setting_name(id).unwrap_or_else(|| format!("0x{id:08X}"));
                    out.push((profile_name.clone(), setting_name, value, predefined));
                }
            }
            if i > 100_000 {
                break;
            }
        }
        unsafe { (self.destroy_session)(s) };
        Ok(out)
    }

    /// The handle of the first profile whose name contains `name_filter`.
    ///
    /// Matching on a fragment rather than the exact name is deliberate: the
    /// driver's own name for a game is whatever NVIDIA typed ("Valorant", not
    /// the executable), and a caller that had to know it exactly would be one
    /// driver update away from silently matching nothing.
    fn find_profile(
        &self,
        s: SessionHandle,
        name_filter: &str,
    ) -> Option<(ProfileHandle, String)> {
        let wanted = name_filter.to_uppercase();
        let mut i = 0u32;
        loop {
            let mut p: ProfileHandle = 0;
            if unsafe { (self.enum_profiles)(s, i, &mut p) } != NVAPI_OK {
                return None;
            }
            i += 1;
            if let Some((name, _)) = self.profile_info(s, p) {
                if name.to_uppercase().contains(&wanted) {
                    return Some((p, name));
                }
            }
            if i > 100_000 {
                return None;
            }
        }
    }

    /// Write settings into a named application profile instead of the global
    /// one.
    ///
    /// This is how one game gets a different answer from every other: the base
    /// profile is the *weakest* entry in the repository, so an application
    /// profile naming the same setting wins for that executable and leaves the
    /// global value untouched for everything else.
    ///
    /// Returns the profile's real name alongside the changes, because a filter
    /// that matched the wrong profile is otherwise indistinguishable from one
    /// that matched the right one.
    pub fn write_u32_in_profile(
        &self,
        name_filter: &str,
        changes: &[(u32, u32)],
    ) -> Result<(String, Vec<(u32, u32, u32)>)> {
        let (s, _) = self.session()?;
        let found = self.find_profile(s, name_filter);
        let Some((p, name)) = found else {
            unsafe { (self.destroy_session)(s) };
            return Err(anyhow!("'{name_filter}' adini iceren bir surucu profili yok"));
        };

        let mut applied = Vec::new();
        for &(id, value) in changes {
            let before = self
                .get_setting_in(s, p, id)
                .filter(|&(_, _, loc)| loc == LOCATION_CURRENT_PROFILE)
                .map(|(v, _, _)| v)
                .unwrap_or(u32::MAX);

            let mut buf = vec![0u8; SETTING_BUF];
            let put = |b: &mut [u8], off: usize, v: u32| {
                b[off..off + 4].copy_from_slice(&v.to_le_bytes());
            };
            put(&mut buf, OFF_VERSION, self.setting_version);
            put(&mut buf, OFF_SETTING_ID, id);
            put(&mut buf, OFF_SETTING_TYPE, TYPE_DWORD);
            put(&mut buf, OFF_CURRENT_VALUE, value);

            if unsafe { (self.set_setting)(s, p, buf.as_mut_ptr()) } == NVAPI_OK {
                let after = self.get_u32_in(s, p, id).map(|(v, _)| v).unwrap_or(u32::MAX);
                applied.push((id, before, after));
            }
        }

        let st = unsafe { (self.save_settings)(s) };
        unsafe { (self.destroy_session)(s) };
        if st != NVAPI_OK {
            return Err(anyhow!("DRS ayarlari kaydedilemedi (durum {st})"));
        }
        Ok((name, applied))
    }

    /// Remove settings from a named application profile.
    ///
    /// The inverse of `write_u32_in_profile`, and deletion rather than a
    /// restore for the same reason it is elsewhere: a setting rogctl added was
    /// not there before, so there is no stock value to go back to. A setting the
    /// profile does not carry counts as already removed.
    pub fn clear_in_profile(&self, name_filter: &str, ids: &[u32]) -> Result<(String, Vec<u32>)> {
        let (s, _) = self.session()?;
        let found = self.find_profile(s, name_filter);
        let Some((p, name)) = found else {
            unsafe { (self.destroy_session)(s) };
            return Err(anyhow!("'{name_filter}' adini iceren bir surucu profili yok"));
        };

        let mut removed = Vec::new();
        for &id in ids {
            let carried = self
                .get_setting_in(s, p, id)
                .is_some_and(|(_, _, loc)| loc == LOCATION_CURRENT_PROFILE);
            if !carried {
                continue;
            }
            if unsafe { (self.delete_setting)(s, p, id) } == NVAPI_OK {
                removed.push(id);
            }
        }

        let st = unsafe { (self.save_settings)(s) };
        unsafe { (self.destroy_session)(s) };
        if st != NVAPI_OK {
            return Err(anyhow!("DRS ayarlari kaydedilemedi (durum {st})"));
        }
        Ok((name, removed))
    }

    /// What one `apply` pass did to a single setting.
    pub fn write_u32(&self, changes: &[(u32, u32)]) -> Result<Vec<(u32, u32, u32)>> {
        let (s, p) = self.session()?;
        let mut applied = Vec::new();

        for &(id, value) in changes {
            let before = self.get_u32_in(s, p, id).map(|(v, _)| v).unwrap_or(u32::MAX);

            let mut buf = vec![0u8; SETTING_BUF];
            let put = |b: &mut [u8], off: usize, v: u32| {
                b[off..off + 4].copy_from_slice(&v.to_le_bytes());
            };
            put(&mut buf, OFF_VERSION, self.setting_version);
            put(&mut buf, OFF_SETTING_ID, id);
            put(&mut buf, OFF_SETTING_TYPE, TYPE_DWORD);
            put(&mut buf, OFF_CURRENT_VALUE, value);

            if unsafe { (self.set_setting)(s, p, buf.as_mut_ptr()) } == NVAPI_OK {
                let after = self.get_u32_in(s, p, id).map(|(v, _)| v).unwrap_or(u32::MAX);
                applied.push((id, before, after));
            }
        }

        let st = unsafe { (self.save_settings)(s) };
        unsafe { (self.destroy_session)(s) };
        if st != NVAPI_OK {
            return Err(anyhow!("DRS ayarlari kaydedilemedi (durum {st})"));
        }
        Ok(applied)
    }

    /// Hand a setting back to whatever the driver shipped as its default.
    ///
    /// Returns the driver's per-setting status rather than a bare count: a
    /// restore that quietly does nothing looks identical to one that worked,
    /// and this call had been silently failing on the base profile for exactly
    /// that reason.
    pub fn restore(&self, ids: &[u32]) -> Result<Vec<(u32, Status)>> {
        let (s, p) = self.session()?;
        let mut results = Vec::new();

        for &id in ids {
            // If the setting is not in this profile (or already gone), it is
            // already at driver default / unmanaged.
            let in_profile = self
                .get_setting_in(s, p, id)
                .is_some_and(|(_, _, loc)| loc == LOCATION_CURRENT_PROFILE);
            if !in_profile {
                results.push((id, NVAPI_OK));
                continue;
            }

            // The dedicated restore entry point is tried first, because it is
            // the only one that can mark a setting genuinely unset rather than
            // explicitly set to the same number. On this driver it answers -9
            // for these ids, so a fallback is required rather than optional.
            let st = unsafe { (self.restore_default)(s, p, id) };
            if st == NVAPI_OK {
                results.push((id, st));
                continue;
            }

            // Next: delete it. A setting rogctl added was absent before, so
            // removing it is what actually returns the profile to its original
            // shape - and the driver confirms there is nothing to restore by
            // reporting no predefined value for these ids.
            let d = unsafe { (self.delete_setting)(s, p, id) };
            if d == NVAPI_OK {
                results.push((id, d));
                continue;
            }

            // Fallback: write back the value the driver itself declares as the
            // default. The setting stays "set", but it is set to stock, which
            // is what the caller actually asked for.
            match self.predefined_u32(s, p, id) {
                Some(def) => {
                    let mut buf = vec![0u8; SETTING_BUF];
                    let put = |b: &mut [u8], off: usize, v: u32| {
                        b[off..off + 4].copy_from_slice(&v.to_le_bytes());
                    };
                    put(&mut buf, OFF_VERSION, self.setting_version);
                    put(&mut buf, OFF_SETTING_ID, id);
                    put(&mut buf, OFF_SETTING_TYPE, TYPE_DWORD);
                    put(&mut buf, OFF_CURRENT_VALUE, def);
                    let w = unsafe { (self.set_setting)(s, p, buf.as_mut_ptr()) };
                    results.push((id, w));
                }
                None => results.push((id, st)),
            }
        }

        let st = unsafe { (self.save_settings)(s) };
        unsafe { (self.destroy_session)(s) };
        if st != NVAPI_OK {
            return Err(anyhow!("DRS geri alma kaydedilemedi (durum {st})"));
        }
        Ok(results)
    }
}

impl Drop for NvApi {
    fn drop(&mut self) {
        unsafe { (self.unload)() };
    }
}

// ---------------------------------------------------------------------------
// The settings we actually drive
// ---------------------------------------------------------------------------

/// The three levers applied here, with the name the driver must confirm.
///
/// Every one of these is a plain integer whose name says exactly what the
/// number means. That is the entire selection criterion. The driver also
/// exposes enum-valued settings - texture filtering quality, power management
/// mode, vertical sync - and those are deliberately left alone: their values
/// are opaque constants, and getting the ordering backwards would quietly
/// select the opposite of what was intended, which for a machine with a heat
/// problem means making it hotter for no visible reason.
pub const FRAME_RATE_LIMITER: (u32, &str) = (0x1083_5002, "Frame Rate Limiter");
pub const IDLE_MAX_FPS: (u32, &str) = (0x1083_5016, "Idle Application Max FPS Limit");
pub const IDLE_TIMEOUT: (u32, &str) = (
    0x1083_5017,
    "Idle Application Threshold Time out in seconds",
);

pub const MANAGED: &[(u32, &str)] = &[FRAME_RATE_LIMITER, IDLE_MAX_FPS, IDLE_TIMEOUT];

/// Read, never written. The panel's variable refresh rate decides which side of
/// the refresh rate the frame cap belongs on, so the state has to be known
/// before a cap can be chosen - but toggling VRR itself is the user's call.
pub const VRR_GLOBAL: (u32, &str) = (0x1094_F157, "Toggle the VRR global feature");

/// Is variable refresh rate switched on for this machine?
///
/// Absent means the driver default is in force. On a G-SYNC laptop panel that
/// default is "on", and the consequence of guessing wrong here is a frame rate
/// that halves, so the safe assumption is the one that keeps the cap below
/// refresh.
pub fn vrr_enabled(nv: &NvApi) -> bool {
    match nv.get_u32(VRR_GLOBAL.0) {
        Some((v, _)) => v != 0,
        None => true,
    }
}

/// The VRR state as the configuration wants it treated.
///
/// `respect_vrr: false` is not "pretend VRR is off" as a workaround - it is the
/// user electing to keep the cap above refresh regardless, and the reason it is
/// a switch rather than a hardcoded rule is that the trade is genuinely theirs:
/// above the VRR window with VSync off costs tearing, not frame rate.
pub fn effective_vrr(nv: &NvApi, respect_vrr: bool) -> bool {
    respect_vrr && vrr_enabled(nv)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snaps_truncated_readings_to_real_rates() {
        // GetDeviceCaps truncates: a 239.76Hz panel reports 239.
        assert_eq!(snap_refresh(239), 240);
        assert_eq!(snap_refresh(143), 144);
        assert_eq!(snap_refresh(59), 60);
        // Exact readings pass through untouched.
        assert_eq!(snap_refresh(144), 144);
        assert_eq!(snap_refresh(240), 240);
    }

    #[test]
    fn leaves_unrecognised_rates_alone() {
        // Far from any known rate: report it rather than inventing one.
        assert_eq!(snap_refresh(37), 37);
        // Unreadable stays unreadable, so callers can refuse to write a cap.
        assert_eq!(snap_refresh(0), 0);
    }

    /// A policy pinned to `hz`, so the test does not depend on this machine's
    /// panel.
    fn policy(max_fps: u32, headroom: u32, hz: u32) -> CapPolicy {
        CapPolicy {
            max_fps,
            headroom,
            vrr_margin: VRR_MARGIN_FPS,
            refresh_hz: hz,
        }
    }

    #[test]
    fn cap_follows_the_vrr_state() {
        // An explicit override wins outright: neither headroom nor VRR touch it.
        assert_eq!(policy(120, 7, 144).resolve(false), 120);
        assert_eq!(policy(120, 7, 144).resolve(true), 120);
        // No override and no readable panel: refuse to invent a cap.
        assert_eq!(
            policy(0, 7, 0).resolve(false) == 0,
            snap_refresh(refresh_hz()) == 0
        );
    }

    #[test]
    fn pinned_refresh_beats_the_panel() {
        // The whole point of the pin: whatever the panel says at this instant,
        // the cap comes off the pinned rate.
        assert_eq!(policy(0, 7, 144).resolve(false), 151);
        assert_eq!(policy(0, 7, 240).resolve(false), 247);
    }

    #[test]
    fn vrr_flips_the_cap_to_the_other_side_of_refresh() {
        // The rule spelled out at 144Hz: 151 without VRR, 141 with it. The sign
        // is the whole point, so assert it at three rates.
        for (hz, headroom) in [(144u32, 7u32), (240, 7), (60, 5)] {
            let off = policy(0, headroom, hz).resolve(false);
            let on = policy(0, headroom, hz).resolve(true);
            assert!(on < hz && hz < off, "{hz}Hz: {on} < {hz} < {off}");
        }
    }
}

/// The refresh rate of the primary display.
///
/// This is what the frame cap is pinned to: frames rendered beyond the panel's
/// refresh rate are never shown, so they are heat and fan noise bought for
/// nothing. Reading it live means the cap follows the panel if the mode is
/// changed, instead of being frozen at whatever it was when this was written.
pub fn refresh_hz() -> u32 {
    #[link(name = "user32")]
    extern "system" {
        fn GetDC(hwnd: isize) -> isize;
        fn ReleaseDC(hwnd: isize, hdc: isize) -> i32;
    }
    #[link(name = "gdi32")]
    extern "system" {
        fn GetDeviceCaps(hdc: isize, index: i32) -> i32;
    }
    const VREFRESH: i32 = 116;

    unsafe {
        let dc = GetDC(0);
        if dc == 0 {
            return 0;
        }
        let hz = GetDeviceCaps(dc, VREFRESH);
        ReleaseDC(0, dc);
        // 0 and 1 both mean "hardware default" for VREFRESH.
        if hz > 1 {
            hz as u32
        } else {
            0
        }
    }
}

/// Refresh rates a real panel actually runs at.
///
/// `GetDeviceCaps(VREFRESH)` truncates rather than rounds, and a panel whose
/// true rate is 239.76Hz or 143.86Hz reports 239 or 143. Building a frame cap
/// on that raw number bakes the rounding error into the driver profile, so the
/// reading is snapped back onto the rate the panel is obviously running.
const KNOWN_REFRESH: &[u32] = &[
    50, 60, 75, 90, 100, 120, 144, 160, 165, 170, 180, 200, 240, 280, 300, 360, 480,
];

/// Distance within which a raw reading is taken to be one of the known rates.
const SNAP_TOLERANCE_HZ: u32 = 2;

/// Snap a raw refresh reading onto the rate the panel is really running.
pub fn snap_refresh(raw: u32) -> u32 {
    if raw == 0 {
        return 0;
    }
    KNOWN_REFRESH
        .iter()
        .copied()
        .find(|&k| k.abs_diff(raw) <= SNAP_TOLERANCE_HZ)
        .unwrap_or(raw)
}

/// How far *below* refresh the cap sits when the panel is running VRR.
///
/// Three frames is NVIDIA's own figure for a G-SYNC panel. The variable refresh
/// window has a ceiling at the panel's maximum rate, and a frame rate that
/// reaches that ceiling drops out of the window: with VSync on the frame waits
/// for the next scan-out, with VSync off (Valorant's case) the screen tears.
/// Staying three frames short keeps the panel inside the window either way.
pub const VRR_MARGIN_FPS: u32 = 3;

/// Everything that decides the frame cap, in one place.
///
/// These arrived one at a time as each was measured to matter, and passing five
/// loose integers in a fixed order to three call sites is how a cap gets written
/// with the headroom in the margin's position. Grouping them also gives the
/// rule a single place to be read.
#[derive(Debug, Clone, Copy)]
pub struct CapPolicy {
    /// Explicit cap. Non-zero wins outright, ignoring everything else here.
    pub max_fps: u32,
    /// Frames above refresh when VRR is not in play.
    pub headroom: u32,
    /// Frames below refresh when it is.
    pub vrr_margin: u32,
    /// Refresh rate to use instead of reading the panel. 0 reads the panel.
    ///
    /// Reading live is right in principle and it is what makes the cap follow a
    /// mode change - but it also means anything that transiently reports a low
    /// rate gets a vote, and the cap it produces is written to a permanent
    /// driver profile. Pinning the number removes that whole class of failure
    /// for a machine whose panel the user is not going to change.
    pub refresh_hz: u32,
}

impl CapPolicy {
    /// The cap to write, or 0 when the panel cannot be read - which callers
    /// must treat as "do not write a cap", since an invented number is worse
    /// than none.
    ///
    /// Which side of the refresh rate the cap belongs on depends entirely on
    /// VRR:
    ///
    /// - **VRR off:** the cap goes *above* refresh. The driver's limiter
    ///   undershoots its target by a handful of frames, so a cap set at refresh
    ///   delivers a rate below refresh, where every dip is visible. Aiming high
    ///   lands the delivered rate on the refresh rate, and the surplus frames
    ///   cost nothing because they are never shown.
    /// - **VRR on:** the cap goes *below* refresh and `headroom` is ignored,
    ///   because above refresh the frame leaves the variable refresh window.
    pub fn resolve(&self, vrr: bool) -> u32 {
        if self.max_fps > 0 {
            return self.max_fps;
        }
        match self.refresh() {
            0 => 0,
            hz if vrr => hz.saturating_sub(self.vrr_margin),
            hz => hz + self.headroom,
        }
    }

    /// The refresh rate this policy is working from: the pinned one if set,
    /// otherwise the panel's, snapped to a rate a panel really runs at.
    pub fn refresh(&self) -> u32 {
        if self.refresh_hz > 0 {
            return self.refresh_hz;
        }
        snap_refresh(refresh_hz())
    }
}

/// One profile's value for a setting.
pub struct ProfileValue {
    pub profile: String,
    /// True when the profile shipped with the driver rather than being created
    /// locally - the distinction between "NVIDIA decided this" and "something on
    /// this machine decided this".
    pub driver_supplied: bool,
    pub value: u32,
    /// True when the value is still the profile's own stock value.
    pub predefined: bool,
}

/// One resolved change, ready to write.
pub struct Change {
    pub id: u32,
    pub name: &'static str,
    pub value: u32,
}

/// Work out what to write, refusing any id whose name the driver does not
/// confirm.
///
/// `max_fps` of 0 means "follow the panel". If the refresh rate cannot be read,
/// the frame cap is skipped rather than guessed - an arbitrary cap would be
/// worse than none.
pub fn plan(
    nv: &NvApi,
    policy: CapPolicy,
    vrr: bool,
    idle_fps: u32,
    idle_timeout_s: u32,
) -> (Vec<Change>, Vec<u32>, Vec<String>) {
    let mut out = Vec::new();
    let mut restore = Vec::new();
    let mut skipped = Vec::new();

    let cap = policy.resolve(vrr);

    let mut want: Vec<(u32, &'static str, u32)> = Vec::new();
    if cap > 0 {
        want.push((FRAME_RATE_LIMITER.0, FRAME_RATE_LIMITER.1, cap));
    } else {
        skipped.push("kare siniri: ekran tazeleme hizi okunamadi, atlandi".to_string());
    }
    // idle_fps 0 means "do not manage this at all". Writing a literal 0 would
    // be a guess about what the driver reads as unlimited, and guessing at a
    // value is exactly what this module refuses to do elsewhere - so the
    // setting is handed back to the driver's own default instead, which is a
    // defined operation with a defined result.
    if idle_fps > 0 {
        want.push((IDLE_MAX_FPS.0, IDLE_MAX_FPS.1, idle_fps));
        want.push((IDLE_TIMEOUT.0, IDLE_TIMEOUT.1, idle_timeout_s));
    } else {
        restore.push(IDLE_MAX_FPS.0);
        restore.push(IDLE_TIMEOUT.0);
    }

    for (id, name, value) in want {
        match nv.setting_name(id) {
            Some(actual) if actual == name => out.push(Change { id, name, value }),
            Some(actual) => skipped.push(format!(
                "0x{id:08X} beklenen '{name}' degil '{actual}' - yazilmadi"
            )),
            None => skipped.push(format!("0x{id:08X} ({name}) bu surucude yok - atlandi")),
        }
    }

    (out, restore, skipped)
}
