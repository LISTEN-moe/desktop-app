use anyhow::Result;
use gpui_kit::Window;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

#[cfg(target_os = "linux")]
#[path = "window/linux.rs"]
mod implementation;
#[cfg(target_os = "windows")]
#[path = "window/windows.rs"]
mod implementation;
#[cfg(target_os = "macos")]
#[path = "window/macos.rs"]
mod implementation;

pub fn configure_frameless(_window: &Window) {
    #[cfg(target_os = "windows")]
    {
        let result = HasWindowHandle::window_handle(_window)
            .map_err(anyhow::Error::from)
            .and_then(|handle| implementation::remove_border(handle.as_raw()));
        if let Err(error) = result {
            eprintln!("Cannot remove the native window border: {error:#}");
        }
    }
}

pub fn supports_always_on_top(window: &Window) -> bool {
    HasWindowHandle::window_handle(window)
        .ok()
        .is_some_and(|handle| implementation::supported(handle.as_raw()))
}

pub fn always_on_top(window: &Window, enabled: bool) -> Result<()> {
    implementation::always_on_top(HasWindowHandle::window_handle(window)?.as_raw(), enabled)
}

pub fn media_window_handle(window: &Window) -> Option<*mut std::ffi::c_void> {
    match HasWindowHandle::window_handle(window).ok()?.as_raw() {
        RawWindowHandle::Win32(handle) => Some(handle.hwnd.get() as *mut std::ffi::c_void),
        _ => None,
    }
}
