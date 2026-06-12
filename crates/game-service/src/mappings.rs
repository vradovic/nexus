pub fn subject_to_hook(subject: &str) -> Option<&'static str> {
    match subject {
        "events.realtime.message" => Some("on_message"),
        _ => None,
    }
}
