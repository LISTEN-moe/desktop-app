use super::state::PlayerState;
use gpui_kit::*;
use listen_moe_desktop::radio::{
    audio::Status,
    visualizer::{Motion, bar_geometry},
};
use std::time::{Duration, Instant};

const FRAME_INTERVAL: Duration = Duration::from_nanos(1_000_000_000 / 60);

pub struct Visualizer {
    model: Entity<PlayerState>,
    motion: Motion,
    started: Instant,
    frame_task: Option<Task<()>>,
    _observer: Subscription,
    _visibility: Subscription,
}
impl Visualizer {
    pub fn new(model: Entity<PlayerState>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let color = model.read(cx).visualizer_accent;
        let observer = cx.observe(&model, |_, _, cx| cx.notify());
        let view = cx.entity().downgrade();
        let visibility = window.observe_window_visibility(move |_, _, cx| {
            let _ = view.update(cx, |_, cx| cx.notify());
        });
        Self {
            model,
            motion: Motion::new(color),
            started: cx.background_executor().now(),
            frame_task: None,
            _observer: observer,
            _visibility: visibility,
        }
    }

    #[cfg(feature = "ui-test")]
    #[allow(dead_code)] // Called by the external GPU integration test.
    pub fn animation_pending(&self) -> bool {
        self.frame_task.is_some()
    }
}
impl Render for Visualizer {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let state = self.model.read(cx);
        let frame = state.visualizer_frame();
        let active = state.playing && matches!(state.audio_status, Status::Playing);
        let now = cx
            .background_executor()
            .now()
            .saturating_duration_since(self.started);
        let presentation = self.motion.sample(
            now,
            frame.as_ref().map(|frame| &frame.bands),
            state.visualizer_accent,
            active,
        );
        let enabled = !cx.reduce_motion() && window.is_visible();
        if enabled && presentation.animating {
            // Match the analysis cadence instead of repainting at 144/240Hz.
            // A single pending timer avoids duplicate loops after interaction.
            if self.frame_task.is_none() {
                let timer = cx.background_executor().timer(FRAME_INTERVAL);
                self.frame_task = Some(cx.spawn(async move |view, cx| {
                    timer.await;
                    let _ = view.update(cx, |this, cx| {
                        this.frame_task = None;
                        cx.notify();
                    });
                }));
            }
        } else {
            self.frame_task = None;
        }
        canvas(
            |_, _, _| (),
            move |bounds, _, window, _| {
                if !enabled || presentation.alpha <= 0. {
                    return;
                }
                let [r, g, b] = presentation.color;
                let color =
                    rgba((u32::from(r) << 24) | (u32::from(g) << 16) | (u32::from(b) << 8) | 255)
                        .opacity(presentation.alpha);
                for bar in bar_geometry(
                    &presentation.bands,
                    bounds.size.width.into(),
                    bounds.size.height.into(),
                    window.scale_factor(),
                ) {
                    window.paint_quad(
                        fill(
                            Bounds::new(
                                bounds.origin + point(px(bar.x), px(bar.y)),
                                size(px(bar.width), px(bar.height)),
                            ),
                            color,
                        )
                        .corner_radii(Corners {
                            top_left: px(bar.radius),
                            top_right: px(bar.radius),
                            bottom_left: px(0.),
                            bottom_right: px(0.),
                        }),
                    );
                }
            },
        )
        .absolute()
        .inset_0()
        .size_full()
    }
}
