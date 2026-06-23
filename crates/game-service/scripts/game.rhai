fn on_message(event, api) {
    let match_channels = [];

    for channel in event.channels {
        if channel != "default" {
            match_channels.push(channel);
        }
    }

    if match_channels.len() > 0 {
        api.broadcast_to_channels(event.payload, match_channels);
    }
}

fn on_match_confirmed(event, api) {
    let channel = "match:" + event.match_id;

    api.create_channel(channel);

    for player_id in event.player_ids {
        api.add_to_channel(player_id, channel);
        api.remove_from_channel(player_id, "default");
    }

    let message = #{};
    message["type"] = "match.found";
    message["match_id"] = event.match_id;
    message["channel"] = channel;
    message["ticket_key"] = event.ticket_key;
    message["player_ids"] = event.player_ids;

    api.broadcast_to_channels(message, [channel]);
}
