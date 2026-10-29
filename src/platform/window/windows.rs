use anyhow::{Result, bail};
use raw_window_handle::RawWindowHandle;
use windows::Win32::{
    Foundation::HWND,
    UI::WindowsAndMessaging::{
        HWND_NOTOPMOST, HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SetWindowPos,
    },
};

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
