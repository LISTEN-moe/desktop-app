use discord_rich_presence::{
    DiscordIpc, DiscordIpcClient,
    activity::{Activity, Assets},
};
use std::sync::mpsc;

pub struct Presence {
    updates: mpsc::Sender<Option<NowPlaying>>,
}

pub struct NowPlaying {
    pub title: String,
    pub artists: String,
    pub channel: &'static str,
}

impl Default for Presence {
    fn default() -> Self {
        let (updates, receiver) = mpsc::channel::<Option<NowPlaying>>();
        std::thread::spawn(move || {
            let mut client = DiscordIpcClient::new("383375119827075072");
            let mut connected = false;
            while let Ok(update) = receiver.recv() {
                let Some(update) = update else {
                    if connected {
                        let _ = client.clear_activity();
                    }
                    continue;
                };
                if !connected {
                    connected = client.connect().is_ok();
                }
                if connected {
                    let title: String = update.title.chars().take(120).collect();
                    let artists: String = update.artists.chars().take(120).collect();
                    let result = client.set_activity(
                        Activity::new().details(title).state(artists).assets(
                            Assets::new()
                                .large_image(update.channel.to_lowercase())
                                .large_text("LISTEN.moe"),
                        ),
                    );
                    if result.is_err() {
                        let _ = client.close();
                        connected = false;
                    }
                }
            }
            if connected {
                let _ = client.clear_activity();
                let _ = client.close();
            }
        });
        Self { updates }
    }
}

impl Presence {
    pub fn update(&self, song: Option<NowPlaying>) {
        let _ = self.updates.send(song);
    }
}

pub fn notify_event(name: String) {
    std::thread::spawn(move || {
        let _ = notify_rust::Notification::new()
            .summary("LISTEN.moe")
            .body(&name)
            .appname("LISTEN.moe")
            .show();
    });
}
