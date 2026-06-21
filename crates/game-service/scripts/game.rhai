fn on_message(event, api) {
    api.broadcast_to_rooms(event.payload, event.rooms);
}
