use anyhow::Result;
use gpui_kit::{App, Pixels, Size, Window};
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

pub fn resize_content(window: &mut Window, size: Size<Pixels>, _cx: &App) -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        let physical = size.to_device_pixels(window.scale_factor());
        let handle = HasWindowHandle::window_handle(window)?.as_raw();
        // As in GPUI's resize implementation, deliver native resize callbacks
        // after the current UI update releases its borrow of the window.
        _cx.foreground_executor()
            .spawn(async move {
                if let Err(error) =
                    implementation::resize_content(handle, physical.width.0, physical.height.0)
                {
                    eprintln!("Cannot resize the player window: {error:#}");
                }
            })
            .detach();
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    {
        window.resize(size);
        Ok(())
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
