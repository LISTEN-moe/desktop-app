use anyhow::{Result, bail};
use raw_window_handle::RawWindowHandle;
use windows::Win32::{
    Foundation::{E_INVALIDARG, ERROR_SUCCESS, GetLastError, HWND, RECT, SetLastError},
    Graphics::Dwm::{
        DWMWA_BORDER_COLOR, DWMWA_COLOR_NONE, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_DONOTROUND,
        DWMWINDOWATTRIBUTE, DwmSetWindowAttribute,
    },
    UI::WindowsAndMessaging::{
        GWL_STYLE, GetClientRect, GetWindowLongPtrW, HWND_NOTOPMOST, HWND_TOPMOST,
        SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, SetWindowLongPtrW,
        SetWindowPos, WINDOW_STYLE, WS_CAPTION, WS_POPUP, WS_THICKFRAME,
    },
};

pub fn remove_border(handle: RawWindowHandle) -> Result<()> {
    let hwnd = win32_handle(handle)?;
    // GPUI creates an overlapped window, which Windows gives an implicit
    // caption/frame. Border color alone leaves its rectangular backdrop intact.
    let style = WINDOW_STYLE(unsafe { GetWindowLongPtrW(hwnd, GWL_STYLE) } as u32);
    let frameless = (style & !(WS_CAPTION | WS_THICKFRAME)) | WS_POPUP;
    if style != frameless {
        let mut client = RECT::default();
        // Preserve the physical client size while removing non-client insets.
        unsafe {
            GetClientRect(hwnd, &mut client)?;
            SetLastError(ERROR_SUCCESS);
            let previous = SetWindowLongPtrW(hwnd, GWL_STYLE, frameless.0 as isize);
            if previous == 0 && GetLastError() != ERROR_SUCCESS {
                return Err(windows::core::Error::from_win32().into());
            }
            SetWindowPos(
                hwnd,
                None,
                0,
                0,
                client.right - client.left,
                client.bottom - client.top,
                SWP_FRAMECHANGED | SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE,
            )?;
        }
    }
    set_dwm_attribute(hwnd, DWMWA_BORDER_COLOR, &DWMWA_COLOR_NONE)?;
    set_dwm_attribute(hwnd, DWMWA_WINDOW_CORNER_PREFERENCE, &DWMWCP_DONOTROUND)
}

fn set_dwm_attribute<T>(hwnd: HWND, attribute: DWMWINDOWATTRIBUTE, value: &T) -> Result<()> {
    // The app owns its rounded surfaces, including the detached artwork. DWM
    // must not draw or round the rectangular native window around them.
    let result = unsafe {
        DwmSetWindowAttribute(
            hwnd,
            attribute,
            (value as *const T).cast(),
            std::mem::size_of_val(value) as u32,
        )
    };
    // Windows 10 does not support this Windows 11 frame attribute.
    match result {
        Err(error) if error.code() != E_INVALIDARG => Err(error.into()),
        _ => Ok(()),
    }
}

pub fn resize_content(handle: RawWindowHandle, width: i32, height: i32) -> Result<()> {
    let hwnd = win32_handle(handle)?;
    // GPUI cached its frame insets before the caption was removed. Resize the
    // now-frameless client directly so artwork changes do not add those back.
    unsafe {
        SetWindowPos(
            hwnd,
            None,
            0,
            0,
            width,
            height,
            SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE,
        )?;
    }
    Ok(())
}

fn win32_handle(handle: RawWindowHandle) -> Result<HWND> {
    match handle {
        RawWindowHandle::Win32(handle) => Ok(HWND(handle.hwnd.get() as *mut _)),
        _ => bail!("Unsupported window handle"),
    }
}

pub fn supported(handle: RawWindowHandle) -> bool {
    matches!(handle, RawWindowHandle::Win32(_))
}

pub fn always_on_top(handle: RawWindowHandle, enabled: bool) -> Result<()> {
    let hwnd = win32_handle(handle)?;
    // The HWND belongs to the live GPUI window supplied by the caller.
    unsafe {
        SetWindowPos(
            hwnd,
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
