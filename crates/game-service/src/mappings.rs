pub fn subject_to_hook(subject: &str) -> Option<&'static str> {
    match subject {
        "events.realtime.message" => Some("on_message"),
        nexus_shared::MATCH_CHANNEL_READY_SUBJECT => Some("on_match_channel_ready"),
        _ => None,
    }
}
