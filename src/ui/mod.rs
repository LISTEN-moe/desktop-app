pub mod preferences;
pub mod state;
pub mod visualizer;
pub mod widget;

pub const SURFACE: u32 = PLAYER_SURFACE;
pub const PLAYER_SURFACE: u32 = 0x292a2f;
pub const MUTED: u32 = 0x989baa;

use gpui_kit::component::{
    Theme, ThemeMode,
    button::{Button, ButtonCustomVariant, ButtonVariants},
    tooltip::Tooltip,
};
use gpui_kit::{
    App, ElementId, Image, ImageFormat, InteractiveElement, MouseButton, ParentElement,
    SharedString, Styled, Svg, px, rgb, rgba,
};
use listen_moe_desktop::radio::model::Channel;
use std::sync::Arc;

pub fn artwork_image(bytes: Vec<u8>) -> Arc<Image> {
    let format = if bytes.starts_with(b"\x89PNG") {
        ImageFormat::Png
    } else if bytes.starts_with(b"GIF8") {
        ImageFormat::Gif
    } else if bytes.starts_with(b"RIFF") {
        ImageFormat::Webp
    } else {
        ImageFormat::Jpeg
    };
    Arc::new(Image::from_bytes(format, bytes))
}

pub fn apply_theme(channel: Channel, cx: &mut App) {
    Theme::change(ThemeMode::Dark, None, cx);
    Theme::update(cx, |theme| {
        theme.colors.background = rgb(SURFACE).into();
        theme.colors.foreground = rgb(0xf0f0f2).into();
        theme.colors.primary = rgb(channel.accent()).into();
        theme.colors.primary_hover = rgb(channel.accent()).into();
        theme.colors.button_primary = rgb(channel.accent()).into();
        theme.colors.button_primary_hover = rgb(channel.accent()).into();
        theme.colors.button_primary_active = rgb(channel.accent()).into();
        theme.colors.button_primary_foreground = rgb(0xffffff).into();
        theme.colors.border = rgba(0xffffff14).into();
        theme.colors.input = rgb(0x383940).into();
        theme.colors.muted_foreground = rgb(MUTED).into();
        theme.colors.switch = rgb(0x3b3c44).into();
        theme.colors.switch_thumb = rgb(0xffffff).into();
        theme.font_size = gpui_kit::px(15.);
    });
}

pub fn action_button(id: impl Into<ElementId>) -> Button {
    Button::new(id)
        .cursor_pointer()
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
}

pub fn icon_button(
    cx: &App,
    id: &'static str,
    icon: Svg,
    label: impl Into<SharedString>,
    color: u32,
) -> Button {
    let label = label.into();
    let mut button = action_button(id)
        .custom(ButtonCustomVariant::new(cx).foreground(rgb(color).into()))
        .group(id)
        .size(px(40.))
        .rounded(px(8.))
        .text_color(rgb(color))
        .accessibility_label(label.clone())
        .child(
            icon.size(px(18.))
                .text_color(rgb(color))
                .group_hover(id, |style| style.text_color(rgb(0xffffff))),
        );
    button
        .interactivity()
        .tooltip(move |window, cx| Tooltip::new(label.clone()).build(window, cx));
    button
}
