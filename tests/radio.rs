use listen_moe_desktop::radio::{
    model::{Channel, GatewayMessage, localized, parse_gateway_message, slider_to_gain},
    preferences::{Preferences, PreferencesStore},
};

#[test]
fn gateway_uses_outer_context_like_v7() {
    let GatewayMessage::Track(song) =
        parse_gateway_message(include_str!("fixtures/track-update.json")).unwrap()
    else {
        panic!("expected track update");
    };
    assert_eq!(song.title(true), "Nihongo no Uta");
    assert_eq!(song.title(false), "日本語の曲");
    assert_eq!(song.artist_names(true), "Character (CV: Singer)");
    assert_eq!(song.start_time.as_deref(), Some("2026-10-28T02:00:00.000Z"));
    assert_eq!(song.requester.as_ref().unwrap().username, "listener");
    assert_eq!(song.context_label().as_deref(), Some("Special broadcast"));
    assert_eq!(
        song.cover_url().as_deref(),
        Some("https://cdn.listen.moe/covers/%E6%97%A5%E6%9C%AC%20%231%3F.jpg")
    );
    assert_eq!(
        song.album_url().as_deref(),
        Some("https://listen.moe/albums/12")
    );
    assert_eq!(
        song.artist_url().as_deref(),
        Some("https://listen.moe/artists/7")
    );
}

#[test]
fn gateway_handles_control_frames_without_creating_tracks() {
    assert!(matches!(
        parse_gateway_message(r#"{"op":10,"d":null}"#).unwrap(),
        GatewayMessage::HeartbeatAck
    ));
    assert!(matches!(
        parse_gateway_message(r#"{"op":1,"t":"QUEUE_UPDATE","d":{"listeners":4}}"#).unwrap(),
        GatewayMessage::Ignore
    ));
    let GatewayMessage::Hello(heartbeat) =
        parse_gateway_message(r#"{"op":0,"d":{"heartbeat":0}}"#).unwrap()
    else {
        panic!("expected hello");
    };
    assert_eq!(heartbeat.as_millis(), 1000);
    assert!(parse_gateway_message("broken json").is_err());
}

#[test]
fn native_and_romaji_names_fall_back_without_blank_labels() {
    assert_eq!(localized(Some("日本語"), None, true), "日本語");
    assert_eq!(localized(None, Some("Romaji"), false), "Romaji");
    assert_eq!(localized(Some(""), Some("Romaji"), false), "Romaji");
    assert_eq!(localized(None, None, true), "Unknown");
}

#[test]
fn volume_matches_v7_curve_and_clamps_input() {
    assert_eq!(slider_to_gain(-50.), 0.);
    assert_eq!(slider_to_gain(0.), 0.);
    assert!((slider_to_gain(50.) - 1. / 3.).abs() < 0.0001);
    assert_eq!(slider_to_gain(100.), 1.);
    assert_eq!(slider_to_gain(200.), 1.);
}

#[test]
fn channels_keep_audio_and_gateway_paired() {
    assert_eq!(Channel::Jpop.gateway_url(), "wss://listen.moe/gateway_v2");
    assert_eq!(Channel::Jpop.stream_url(), "https://listen.moe/fallback");
    assert_eq!(
        Channel::Kpop.gateway_url(),
        "wss://listen.moe/kpop/gateway_v2"
    );
    assert_eq!(
        Channel::Kpop.stream_url(),
        "https://listen.moe/kpop/fallback"
    );
}

#[test]
fn preferences_round_trip_replace_atomically_and_accept_missing_fields() {
    let directory = tempfile::tempdir().unwrap();
    let store = PreferencesStore::at(directory.path().join("config/preferences.json"));
    assert_eq!(store.load().unwrap().volume, 50.);
    let mut preferences = Preferences {
        channel: Channel::Kpop,
        volume: 37.,
        ..Default::default()
    };
    store.save(&preferences).unwrap();
    preferences.volume = 0.;
    store.save(&preferences).unwrap();
    let loaded = store.load().unwrap();
    assert_eq!(loaded.channel, Channel::Kpop);
    assert_eq!(loaded.volume, 0.);
    std::fs::write(
        store.path(),
        r#"{"volume":500,"prefer_romaji":false,"spaced_controls":true,"glass":true}"#,
    )
    .unwrap();
    let loaded = store.load().unwrap();
    assert_eq!(loaded.volume, 100.);
    assert!(!loaded.prefer_romaji);
    assert_eq!(loaded.widget_height(), 72.);
    store.save(&loaded).unwrap();
    let saved = std::fs::read_to_string(store.path()).unwrap();
    for removed_field in ["spaced_controls", "glass"] {
        assert!(
            !saved.contains(removed_field),
            "removed preference {removed_field} is ignored and no longer saved"
        );
    }
}

#[test]
fn corrupt_preferences_report_errors_without_overwriting_the_file() {
    let directory = tempfile::tempdir().unwrap();
    let store = PreferencesStore::at(directory.path().join("preferences.json"));
    std::fs::write(store.path(), "broken json").unwrap();
    assert!(store.load().is_err());
    assert_eq!(
        std::fs::read_to_string(store.path()).unwrap(),
        "broken json"
    );
}
