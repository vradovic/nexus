pub fn subject_to_hook(subject: &str) -> Option<&'static str> {
    match subject {
        "events.cli.hello_world" => Some("on_hello_world"),
        "events.cli.foo" => Some("on_foo"),
        _ => None,
    }
}
