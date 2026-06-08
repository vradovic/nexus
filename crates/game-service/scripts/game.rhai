fn on_hello_world(payload, api) {
    print("hello " + payload.name);
}

fn on_foo(payload, api) {
    api.broadcast("baz");
}
