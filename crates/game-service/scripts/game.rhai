fn on_message(event, api) {
    let match_rooms = [];

    for room in event.rooms {
        if room != "default" {
            match_rooms.push(room);
        }
    }

    if match_rooms.len() > 0 {
        api.broadcast_to_rooms(event.payload, match_rooms);
    }
}

fn on_match_confirmed(event, api) {
    let room = "match:" + event.match_id;

    api.create_room(room);

    for player_id in event.player_ids {
        api.add_to_room(player_id, room);
        api.remove_from_room(player_id, "default");
    }

    let message = #{};
    message["type"] = "match.found";
    message["match_id"] = event.match_id;
    message["room"] = room;
    message["ticket_key"] = event.ticket_key;
    message["player_ids"] = event.player_ids;

    api.broadcast_to_rooms(message, [room]);
}
