use anyhow::{Context, Result, bail};
use objc2_app_kit::NSView;
use raw_window_handle::RawWindowHandle;

pub fn supported(handle: RawWindowHandle) -> bool {
    matches!(handle, RawWindowHandle::AppKit(_))
}

pub fn always_on_top(handle: RawWindowHandle, enabled: bool) -> Result<()> {
    let RawWindowHandle::AppKit(handle) = handle else {
        bail!("Unsupported window handle");
    };
    // GPUI supplies a live NSView and invokes this on AppKit's main thread.
    let view = unsafe { &*handle.ns_view.as_ptr().cast::<NSView>() };
    let window = view.window().context("The view has no native window")?;
    window.setLevel(if enabled { 3 } else { 0 });
    Ok(())
}
