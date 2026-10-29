use anyhow::{Result, bail};
use raw_window_handle::RawWindowHandle;
use windows::Win32::{
    Foundation::{E_INVALIDARG, HWND},
    Graphics::Dwm::{DWMWA_BORDER_COLOR, DWMWA_COLOR_NONE, DwmSetWindowAttribute},
    UI::WindowsAndMessaging::{
        HWND_NOTOPMOST, HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SetWindowPos,
    },
};

pub fn remove_border(handle: RawWindowHandle) -> Result<()> {
    let RawWindowHandle::Win32(handle) = handle else {
        bail!("Unsupported window handle");
    };
    // The app paints its own surfaces. Suppress DWM's border around the entire
    // native window, including the widget's transparent artwork gap.
    let result = unsafe {
        DwmSetWindowAttribute(
            HWND(handle.hwnd.get() as *mut _),
            DWMWA_BORDER_COLOR,
            (&DWMWA_COLOR_NONE as *const u32).cast(),
            std::mem::size_of::<u32>() as u32,
        )
    };
    // Windows 10 does not support this Windows 11 frame attribute.
    match result {
        Err(error) if error.code() != E_INVALIDARG => Err(error.into()),
        _ => Ok(()),
    }
}

pub fn supported(handle: RawWindowHandle) -> bool {
    matches!(handle, RawWindowHandle::Win32(_))
}

pub fn always_on_top(handle: RawWindowHandle, enabled: bool) -> Result<()> {
    let RawWindowHandle::Win32(handle) = handle else {
        bail!("Unsupported window handle");
    };
    // The HWND belongs to the live GPUI window supplied by the caller.
    unsafe {
        SetWindowPos(
            HWND(handle.hwnd.get() as *mut _),
            Some(if enabled {
                HWND_TOPMOST
            } else {
                HWND_NOTOPMOST
            }),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        )?;
    }
    Ok(())
}
