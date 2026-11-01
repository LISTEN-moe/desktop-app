use anyhow::{Result, bail};
use raw_window_handle::RawWindowHandle;
use x11rb::{
    connection::Connection,
    protocol::xproto::{ClientMessageEvent, ConnectionExt, EventMask},
};

pub fn supported(handle: RawWindowHandle) -> bool {
    matches!(handle, RawWindowHandle::Xlib(_) | RawWindowHandle::Xcb(_))
}

pub fn always_on_top(handle: RawWindowHandle, enabled: bool) -> Result<()> {
    let id = match handle {
        RawWindowHandle::Xlib(handle) => u32::try_from(handle.window)?,
        RawWindowHandle::Xcb(handle) => handle.window.get(),
        _ => bail!("Always on top is controlled by the Wayland compositor"),
    };
    let (connection, screen) = x11rb::connect(None)?;
    let root = connection.setup().roots[screen].root;
    let state = connection
        .intern_atom(false, b"_NET_WM_STATE")?
        .reply()?
        .atom;
    let above = connection
        .intern_atom(false, b"_NET_WM_STATE_ABOVE")?
        .reply()?
        .atom;
    let event = ClientMessageEvent::new(32, id, state, [u32::from(enabled), above, 0, 1, 0]);
    connection.send_event(
        false,
        root,
        EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY,
        event,
    )?;
    connection.flush()?;
    Ok(())
}
