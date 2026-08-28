//! ASUS ATKACPI bridge.
//!
//! Armoury Crate and every other ASUS control utility ultimately talk to the
//! same entry point: the `\\.\ATKACPI` device exposes one ioctl that forwards
//! `DSTS` (read device status) and `DEVS` (write device control) calls into
//! the BIOS. Fans, CPU power limits, the dGPU mux and the performance mode all
//! live behind it, so this module is the foundation the rest of the daemon
//! stands on.
//!
//! Opening the device requires administrator rights.

use std::ffi::c_void;
use std::io;
use std::ptr;

use windows_sys::Win32::Foundation::{CloseHandle, GENERIC_READ, GENERIC_WRITE, HANDLE, INVALID_HANDLE_VALUE};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
};
use windows_sys::Win32::System::IO::DeviceIoControl;

/// The single ioctl every ASUS ACPI method call rides on.
const IOCTL_ASUS_WMI_METHOD: u32 = 0x0022_240C;

/// Method selectors, four ASCII bytes read as a little-endian u32.
pub const METHOD_DSTS: u32 = 0x5354_5344; // "DSTS" - read device status
pub const METHOD_DEVS: u32 = 0x5356_4544; // "DEVS" - write device control
pub const METHOD_INIT: u32 = 0x5449_4E49; // "INIT" - handshake

/// A `DSTS` reply with this bit clear means the BIOS does not implement the
/// device at all, as opposed to implementing it and reporting zero.
pub const STATUS_SUPPORTED: u32 = 0x0001_0000;

/// Interpret the first four bytes of a BIOS reply as a status word.
fn scalar(out: Vec<u8>) -> anyhow::Result<u32> {
    if out.len() < 4 {
        anyhow::bail!("kisa yanit: {} bayt", out.len());
    }
    Ok(u32::from_le_bytes([out[0], out[1], out[2], out[3]]))
}

pub struct Acpi {
    handle: HANDLE,
}

// The handle is only ever used through &mut self via DeviceIoControl, which is
// itself thread-safe; the raw pointer is what makes the compiler hesitate.
unsafe impl Send for Acpi {}

impl Acpi {
    /// Open `\\.\ATKACPI`. Fails without administrator rights.
    pub fn open() -> anyhow::Result<Self> {
        // UTF-16, NUL terminated: \\.\ATKACPI
        let path: Vec<u16> = r"\\.\ATKACPI".encode_utf16().chain(std::iter::once(0)).collect();

        let handle = unsafe {
            CreateFileW(
                path.as_ptr(),
                GENERIC_READ | GENERIC_WRITE,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                ptr::null(),
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL,
                ptr::null_mut(),
            )
        };

        if handle == INVALID_HANDLE_VALUE || handle.is_null() {
            let err = io::Error::last_os_error();
            anyhow::bail!(
                "\\\\.\\ATKACPI acilamadi ({err}). \
                 Yonetici olarak calistirilmasi gerekiyor ve ASUS System Control Interface \
                 surucusunun yuklu olmasi lazim."
            );
        }

        Ok(Self { handle })
    }

    /// Raw method call. The ACPI buffer layout is
    /// `[method_id: u32][args_len: u32][args...]`.
    ///
    /// Returns exactly the bytes the BIOS wrote back. Scalar devices answer
    /// with four; fan curves answer with sixteen.
    pub fn call(&self, method: u32, args: &[u8]) -> anyhow::Result<Vec<u8>> {
        let mut input = Vec::with_capacity(8 + args.len());
        input.extend_from_slice(&method.to_le_bytes());
        input.extend_from_slice(&(args.len() as u32).to_le_bytes());
        input.extend_from_slice(args);

        let mut output = [0u8; 64];
        let mut returned: u32 = 0;

        let ok = unsafe {
            DeviceIoControl(
                self.handle,
                IOCTL_ASUS_WMI_METHOD,
                input.as_ptr() as *const c_void,
                input.len() as u32,
                output.as_mut_ptr() as *mut c_void,
                output.len() as u32,
                &mut returned,
                ptr::null_mut(),
            )
        };

        if ok == 0 {
            anyhow::bail!("DeviceIoControl basarisiz: {}", io::Error::last_os_error());
        }

        let n = (returned as usize).min(output.len());
        Ok(output[..n].to_vec())
    }

    /// Read a device's current status as a scalar.
    pub fn read(&self, device_id: u32) -> anyhow::Result<u32> {
        let out = self.call(METHOD_DSTS, &device_id.to_le_bytes())?;
        if out.len() < 4 {
            anyhow::bail!("kisa yanit: {} bayt", out.len());
        }
        Ok(u32::from_le_bytes([out[0], out[1], out[2], out[3]]))
    }

    /// Read a device's full reply. Fan curve devices return sixteen bytes:
    /// eight temperature points followed by eight fan-speed points.
    pub fn read_blob(&self, device_id: u32) -> anyhow::Result<Vec<u8>> {
        self.call(METHOD_DSTS, &device_id.to_le_bytes())
    }

    /// Write a scalar value to a device.
    pub fn write(&self, device_id: u32, value: u32) -> anyhow::Result<u32> {
        let mut args = [0u8; 8];
        args[..4].copy_from_slice(&device_id.to_le_bytes());
        args[4..].copy_from_slice(&value.to_le_bytes());
        scalar(self.call(METHOD_DEVS, &args)?)
    }

    /// Write a byte blob to a device. Fan curves use this with 16 bytes:
    /// eight temperature points followed by eight fan-speed points.
    pub fn write_blob(&self, device_id: u32, blob: &[u8]) -> anyhow::Result<u32> {
        let mut args = Vec::with_capacity(4 + blob.len());
        args.extend_from_slice(&device_id.to_le_bytes());
        args.extend_from_slice(blob);
        scalar(self.call(METHOD_DEVS, &args)?)
    }

    /// BIOS handshake. Some firmware revisions refuse control writes until an
    /// application has announced itself.
    pub fn init(&self) -> anyhow::Result<u32> {
        scalar(self.call(METHOD_INIT, &0u32.to_le_bytes())?)
    }
}

impl Drop for Acpi {
    fn drop(&mut self) {
        if !self.handle.is_null() && self.handle != INVALID_HANDLE_VALUE {
            unsafe { CloseHandle(self.handle) };
        }
    }
}
