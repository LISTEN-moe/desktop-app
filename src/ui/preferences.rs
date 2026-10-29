use super::{MUTED, SURFACE, action_button, icon_button, state::PlayerState, widget::Quit};
use gpui_kit::component::{
    button::{Button, ButtonCustomVariant, ButtonVariants},
    input::{Input, InputEvent, InputState},
    switch::Switch,
    *,
};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;
use listen_moe_desktop::radio::{model::Channel, preferences::Preferences};

gpui_kit::actions!(preferences, [ClosePreferences]);

pub fn open(model: Entity<PlayerState>, cx: &mut App) -> anyhow::Result<()> {
    if let Some(handle) = model.read(cx).preferences_window
        && handle
            .update(cx, |_, window, _| window.activate_window())
            .is_ok()
    {
        return Ok(());
    }
    let (handle, _) = gpui_kit::open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::centered(size(px(440.), px(700.)), cx)),
            titlebar: None,
            is_resizable: false,
            window_min_size: Some(size(px(360.), px(520.))),
            app_id: Some("moe.listen.desktop".into()),
            ..Default::default()
        },
        cx,
        |window, cx| cx.new(|cx| PreferencesWindow::new(model.clone(), window, cx)),
    )?;
    model.update(cx, |state, cx| {
        state.preferences_window = Some(handle);
        cx.notify();
    });
    Ok(())
}

pub struct PreferencesWindow {
    model: Entity<PlayerState>,
    username: Entity<InputState>,
    password: Entity<InputState>,
    code: Entity<InputState>,
    focus: FocusHandle,
    sign_in_open: bool,
    scroll: ScrollHandle,
    _subscriptions: Vec<Subscription>,
}

impl PreferencesWindow {
    fn new(model: Entity<PlayerState>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        window.set_window_title("LISTEN.moe Preferences");
        let username = cx.new(|cx| InputState::new(window, cx).placeholder("Username"));
        let password = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Password")
                .masked(true)
        });
        let code = cx.new(|cx| InputState::new(window, cx).placeholder("Six-digit code"));
        let mut subscriptions = vec![cx.observe(&model, |_, _, cx| cx.notify())];
        for input in [&username, &password, &code] {
            subscriptions.push(cx.subscribe_in(
                input,
                window,
                |this, _, event: &InputEvent, window, cx| {
                    if matches!(event, InputEvent::PressEnter { .. }) {
                        this.sign_in(window, cx);
                    }
                },
            ));
        }
        let focus = cx.focus_handle();
        window.focus(&focus, cx);
        Self {
            model,
            username,
            password,
            code,
            focus,
            sign_in_open: false,
            scroll: ScrollHandle::default(),
            _subscriptions: subscriptions,
        }
    }

    fn sign_in(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.model.read(cx).temporary_token.is_some() {
            let code = self.code.read(cx).value().trim().to_owned();
            self.model
                .update(cx, |state, cx| state.verify_mfa(code, cx));
            self.code
                .update(cx, |input, cx| input.set_value("", window, cx));
        } else {
            let username = self.username.read(cx).value().trim().to_owned();
            let password = self.password.read(cx).value().to_string();
            self.model
                .update(cx, |state, cx| state.login(username, password, cx));
            self.password
                .update(cx, |input, cx| input.set_value("", window, cx));
        }
    }

    fn update_preference(
        &mut self,
        setter: fn(&mut Preferences, bool),
        value: bool,
        cx: &mut Context<Self>,
    ) {
        self.model.update(cx, |state, cx| {
            setter(&mut state.preferences, value);
            state.save_preferences(cx);
        });
    }

    fn toggle(
        &self,
        id: &'static str,
        label: &'static str,
        checked: bool,
        setter: fn(&mut Preferences, bool),
        enabled: bool,
        cx: &Context<Self>,
    ) -> AnyElement {
        div()
            .id(SharedString::from(format!("{id}-row")))
            .test_support()
            .flex()
            .items_center()
            .justify_between()
            .min_h(px(48.))
            .px(px(14.))
            .py(px(4.))
            .gap(px(16.))
            .when(enabled, |row| {
                row.cursor_pointer()
                    .hover(|style| style.text_color(rgb(0xffffff)))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.update_preference(setter, !checked, cx);
                    }))
            })
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap(px(2.))
                    .when(!enabled, |label| label.text_color(rgb(MUTED)))
                    .child(label)
                    .when(!enabled, |label| {
                        label.child(
                            div()
                                .text_size(px(12.))
                                .line_height(px(18.))
                                .text_color(rgb(MUTED))
                                .child("Set in your window manager"),
                        )
                    }),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_end()
                    .w(px(44.))
                    .h(px(40.))
                    .flex_shrink_0()
                    .child(
                        Switch::new(id)
                            .checked(checked)
                            .color(rgb(0x737680))
                            .disabled(!enabled)
                            .when(enabled, |switch| switch.cursor_pointer())
                            .accessibility_label(label)
                            .on_change(cx.listener(move |this, value, _, cx| {
                                cx.stop_propagation();
                                this.update_preference(setter, *value, cx);
                            })),
                    ),
            )
            .into_any_element()
    }
}

fn account_input(
    id: &'static str,
    label: &'static str,
    state: &Entity<InputState>,
) -> impl IntoElement {
    div()
        .id(id)
        .test_support()
        .flex()
        .flex_col()
        .gap(px(6.))
        .child(
            div()
                .text_size(px(12.))
                .line_height(px(18.))
                .text_color(rgb(MUTED))
                .child(label),
        )
        .child(Input::new(state).large())
}

fn group(id: &'static str, title: &'static str) -> impl IntoElement + ParentElement {
    div()
        .id(id)
        .test_support()
        .flex()
        .flex_col()
        .gap(px(8.))
        .child(
            div()
                .text_size(px(13.))
                .line_height(px(18.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(rgb(MUTED))
                .child(title),
        )
}

fn card() -> Div {
    div().flex().flex_col().bg(rgb(0x222328)).rounded(px(12.))
}

fn divider() -> Div {
    div()
        .mx(px(14.))
        .h(px(1.))
        .bg(rgba(0xffffff0b))
        .flex_shrink_0()
}

fn neutral_button(id: &'static str, label: &'static str, cx: &App) -> Button {
    action_button(id)
        .label(label)
        .bg(rgb(0x3b3c44))
        .h(px(40.))
        .px(px(14.))
        .rounded(px(8.))
        .custom(
            ButtonCustomVariant::new(cx)
                .color(rgb(0x3b3c44).into())
                .foreground(rgb(0xf0f0f2).into())
                .hover(rgb(0x464750).into())
                .active(rgb(0x34353d).into()),
        )
}

fn quiet_button(id: &'static str, label: &'static str, cx: &App) -> Button {
    action_button(id)
        .group(id)
        .h(px(40.))
        .px(px(8.))
        .rounded(px(8.))
        .custom(ButtonCustomVariant::new(cx))
        .child(
            div()
                .text_size(px(13.))
                .text_color(rgb(MUTED))
                .group_hover(id, |style| style.text_color(rgb(0xffffff)))
                .child(label),
        )
}

impl Render for PreferencesWindow {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let state = self.model.read(cx);
        let preferences = state.preferences.clone();
        let channel = preferences.channel;
        let can_pin = state.can_pin;
        let auth_pending = state.auth_pending;
        let mfa = state.temporary_token.is_some();
        let user = state
            .account
            .as_ref()
            .map(|account| account.user.display_name.clone());
        let signed_in = user.is_some();
        let show_form = !signed_in && (self.sign_in_open || mfa);
        let notice = state.notice.clone();

        let account = group("account-section", "Account").child(
            card()
                .p(px(14.))
                .gap(px(14.))
                .when(!show_form, |element| {
                    element.child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap(px(12.))
                            .child(
                                div()
                                    .min_w_0()
                                    .flex_1()
                                    .truncate()
                                    .text_color(rgb(if signed_in { 0xf0f0f2 } else { MUTED }))
                                    .child(user.unwrap_or_else(|| "Not signed in".into())),
                            )
                            .when(signed_in, |element| {
                                element.child(quiet_button("sign-out", "Sign out", cx).on_click(
                                    cx.listener(|this, _, _, cx| {
                                        this.sign_in_open = false;
                                        this.model.update(cx, |state, cx| state.logout(cx));
                                    }),
                                ))
                            })
                            .when(!signed_in, |element| {
                                element.child(
                                    neutral_button("show-sign-in", "Sign in", cx).on_click(
                                        cx.listener(|this, _, window, cx| {
                                            this.sign_in_open = true;
                                            this.scroll.scroll_to_bottom();
                                            this.username
                                                .read(cx)
                                                .focus_handle(cx)
                                                .focus(window, cx);
                                            cx.notify();
                                        }),
                                    ),
                                )
                            }),
                    )
                })
                .when(show_form, |element| {
                    element
                        .when(!mfa, |element| {
                            element
                                .child(account_input("username-field", "Username", &self.username))
                                .child(account_input("password-field", "Password", &self.password))
                        })
                        .when(mfa, |element| {
                            element
                                .child(
                                    div()
                                        .text_size(px(13.))
                                        .text_color(rgb(MUTED))
                                        .child("Two-factor authentication"),
                                )
                                .child(account_input("code-field", "Verification code", &self.code))
                        })
                        .child(
                            neutral_button("sign-in", if mfa { "Verify" } else { "Sign in" }, cx)
                                .w_full()
                                .bg(rgb(0xe1e2e8))
                                .custom(
                                    ButtonCustomVariant::new(cx)
                                        .color(rgb(0xe1e2e8).into())
                                        .foreground(rgb(0x25262b).into())
                                        .hover(rgb(0xffffff).into())
                                        .active(rgb(0xced0d8).into()),
                                )
                                .loading(auth_pending)
                                .on_click(
                                    cx.listener(|this, _, window, cx| this.sign_in(window, cx)),
                                ),
                        )
                        .child(
                            div()
                                .flex()
                                .justify_between()
                                .gap(px(8.))
                                .child(
                                    quiet_button(
                                        "create-account",
                                        if mfa {
                                            "Back to sign in"
                                        } else {
                                            "Create account ↗"
                                        },
                                        cx,
                                    )
                                    .on_click(cx.listener(
                                        move |this, _, window, cx| {
                                            if mfa {
                                                this.sign_in_open = true;
                                                this.scroll.scroll_to_bottom();
                                                this.code.update(cx, |input, cx| {
                                                    input.set_value("", window, cx)
                                                });
                                                this.model.update(cx, |state, cx| state.logout(cx));
                                            } else {
                                                cx.open_url("https://listen.moe/");
                                            }
                                        },
                                    )),
                                )
                                .when(!mfa, |element| {
                                    element.child(
                                        quiet_button("cancel-sign-in", "Cancel", cx).on_click(
                                            cx.listener(|this, _, window, cx| {
                                                this.sign_in_open = false;
                                                this.password.update(cx, |input, cx| {
                                                    input.set_value("", window, cx)
                                                });
                                                cx.notify();
                                            }),
                                        ),
                                    )
                                }),
                        )
                }),
        );

        div()
            .id("preferences-window")
            .test_support()
            .key_context("Preferences")
            .track_focus(&self.focus)
            .on_action(|_: &ClosePreferences, window, _| window.remove_window())
            .on_action(|_: &Quit, _, cx| cx.quit())
            .size_full()
            .flex()
            .flex_col()
            .gap(px(20.))
            .bg(rgb(SURFACE))
            .text_color(rgb(0xe1e2e8))
            .text_size(px(14.))
            .line_height(px(20.))
            .p(px(24.))
            .child(
                div()
                    .id("preferences-drag")
                    .test_support()
                    .flex()
                    .items_center()
                    .justify_between()
                    .flex_shrink_0()
                    .h(px(40.))
                    .cursor_move()
                    .on_mouse_down(MouseButton::Left, |_, window, _| window.start_window_move())
                    .child(
                        div()
                            .text_size(px(20.))
                            .line_height(px(26.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(rgb(0xf0f0f2))
                            .child("Preferences"),
                    )
                    .child(
                        icon_button(
                            cx,
                            "close-preferences",
                            svg().path(assets::IconName::Close.path()),
                            "Close preferences",
                            MUTED,
                        )
                        .on_click(|_, window, _| window.remove_window()),
                    ),
            )
            .child(
                div()
                    .id("preferences-content")
                    .test_support()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .flex_col()
                    .gap(px(16.))
                    .overflow_y_scroll()
                    .track_scroll(&self.scroll)
                    .child(
                        group("station-section", "Station").child(
                            div()
                                .flex()
                                .gap(px(4.))
                                .p(px(4.))
                                .bg(rgb(0x222328))
                                .rounded(px(12.))
                                .children([Channel::Jpop, Channel::Kpop].map(|radio| {
                                    let selected = channel == radio;
                                    action_button(radio.label())
                                        .h(px(40.))
                                        .flex_1()
                                        .rounded(px(8.))
                                        .selected(selected)
                                        .toggled(selected)
                                        .custom(
                                            ButtonCustomVariant::new(cx)
                                                .color(if selected {
                                                    rgb(0x3b3c44).into()
                                                } else {
                                                    rgba(0).into()
                                                })
                                                .foreground(
                                                    rgb(if selected { 0xf0f0f2 } else { MUTED })
                                                        .into(),
                                                )
                                                .hover(rgb(0x3b3c44).into())
                                                .active(rgb(0x464750).into()),
                                        )
                                        .child(
                                            div()
                                                .flex()
                                                .items_center()
                                                .gap(px(8.))
                                                .when(selected, |element| {
                                                    element.child(
                                                        div()
                                                            .size(px(6.))
                                                            .rounded_full()
                                                            .bg(rgb(radio.accent())),
                                                    )
                                                })
                                                .child(radio.label()),
                                        )
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            this.model.update(cx, |state, cx| {
                                                state.set_channel(radio, cx)
                                            });
                                        }))
                                })),
                        ),
                    )
                    .child(
                        group("appearance-section", "Appearance").child(
                            card()
                                .child(self.toggle(
                                    "romaji",
                                    "Prefer romaji",
                                    preferences.prefer_romaji,
                                    |preferences, value| preferences.prefer_romaji = value,
                                    true,
                                    cx,
                                ))
                                .child(divider())
                                .child(self.toggle(
                                    "artwork",
                                    "Large artwork",
                                    preferences.large_artwork,
                                    |preferences, value| preferences.large_artwork = value,
                                    true,
                                    cx,
                                )),
                        ),
                    )
                    .child(
                        group("desktop-section", "Desktop").child(
                            card()
                                .child(self.toggle(
                                    "topmost",
                                    "Always on top",
                                    preferences.always_on_top && can_pin,
                                    |preferences, value| preferences.always_on_top = value,
                                    can_pin,
                                    cx,
                                ))
                                .child(divider())
                                .child(self.toggle(
                                    "notifications",
                                    "Event notifications",
                                    preferences.event_notifications,
                                    |preferences, value| preferences.event_notifications = value,
                                    true,
                                    cx,
                                ))
                                .child(divider())
                                .child(self.toggle(
                                    "discord",
                                    "Discord presence",
                                    preferences.discord_presence,
                                    |preferences, value| preferences.discord_presence = value,
                                    true,
                                    cx,
                                )),
                        ),
                    )
                    .child(account)
                    .when_some(notice, |element, message| {
                        element.child(
                            div()
                                .id("status-message")
                                .test_support()
                                .p(px(12.))
                                .rounded(px(8.))
                                .bg(rgb(0x343238))
                                .text_color(rgb(0xd8bf9c))
                                .text_size(px(12.))
                                .line_height(px(18.))
                                .child(message),
                        )
                    }),
            )
            .child(
                div()
                    .id("preferences-footer")
                    .test_support()
                    .flex()
                    .justify_between()
                    .items_center()
                    .flex_shrink_0()
                    .border_t_1()
                    .border_color(rgba(0xffffff0d))
                    .pt(px(12.))
                    .child(
                        quiet_button("open-website", "LISTEN.moe ↗", cx)
                            .on_click(|_, _, cx| cx.open_url("https://listen.moe/")),
                    )
                    .child(quiet_button("quit", "Quit", cx).on_click(|_, _, cx| cx.quit())),
            )
    }
}
