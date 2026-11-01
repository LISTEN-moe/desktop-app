#![cfg(target_os = "windows")]

#[path = "../src/platform/window/windows.rs"]
#[allow(dead_code)]
mod window;

use anyhow::Result;
use raw_window_handle::{RawWindowHandle, Win32WindowHandle};
use std::num::NonZeroIsize;
use windows::{
    Win32::{
        Foundation::{HWND, RECT},
        UI::WindowsAndMessaging::{
            CreateWindowExW, DestroyWindow, GWL_STYLE, GetClientRect, GetWindowLongPtrW,
            WINDOW_STYLE, WS_CAPTION, WS_EX_APPWINDOW, WS_EX_NOREDIRECTIONBITMAP, WS_MINIMIZEBOX,
            WS_POPUP, WS_SYSMENU, WS_THICKFRAME,
        },
    },
    core::w,
};

struct NativeWindow(HWND);

impl NativeWindow {
    fn new(style: WINDOW_STYLE) -> Result<Self> {
        // The built-in class needs no GPU or custom window procedure. Use the
        // same top-level styles as GPUI to exercise Windows' implicit caption.
        let handle = unsafe {
            CreateWindowExW(
                WS_EX_APPWINDOW | WS_EX_NOREDIRECTIONBITMAP,
                w!("STATIC"),
                w!("LISTEN.moe frame test"),
                style,
                0,
                0,
                560,
                88,
                None,
                None,
                None,
                None,
            )?
        };
        Ok(Self(handle))
    }

    fn raw_handle(&self) -> RawWindowHandle {
        Win32WindowHandle::new(NonZeroIsize::new(self.0.0 as isize).unwrap()).into()
    }

    fn style(&self) -> WINDOW_STYLE {
        WINDOW_STYLE(unsafe { GetWindowLongPtrW(self.0, GWL_STYLE) } as u32)
    }

    fn client_size(&self) -> Result<(i32, i32)> {
        let mut rect = RECT::default();
        unsafe { GetClientRect(self.0, &mut rect)? };
        Ok((rect.right - rect.left, rect.bottom - rect.top))
    }
}

impl Drop for NativeWindow {
    fn drop(&mut self) {
        let _ = unsafe { DestroyWindow(self.0) };
    }
}

#[test]
fn frameless_windows_remove_the_caption_and_preserve_client_dimensions() -> Result<()> {
    for style in [WS_SYSMENU, WS_SYSMENU | WS_MINIMIZEBOX] {
        let native = NativeWindow::new(style)?;
        assert_ne!((native.style() & WS_CAPTION).0, 0);
        let original_size = native.client_size()?;

        window::remove_border(native.raw_handle())?;
        let frameless = native.style();
        assert_eq!((frameless & (WS_CAPTION | WS_THICKFRAME)).0, 0);
        assert_ne!((frameless & WS_POPUP).0, 0);
        assert_eq!((frameless & style).0, style.0);
        assert_eq!(native.client_size()?, original_size);

        window::remove_border(native.raw_handle())?;
        assert_eq!(native.client_size()?, original_size);

        for size in [(560, 72), (560, 88), (840, 132), (560, 72)] {
            window::resize_content(native.raw_handle(), size.0, size.1)?;
            assert_eq!(native.client_size()?, size);
        }
    }
    Ok(())
}
