fn on_message(event, api) {
    let channel = match_channel(event.channels);
    if channel == "" {
        return;
    }

    let message = event.message;
    if message == () {
        return;
    }

    let message_type = message["type"];
    if message_type == "chess.sync_request" {
        let state = load_state_for_message(message, api);
        if state != () && state["channel"] == channel {
            let reply = build_position_message(state, "sync_response", event.connection_id, message["clientName"], (), "");
            let reply_channel = state["channel"];
            api.broadcast_to_channels(reply, [reply_channel]);
        }
        return;
    }

    if message_type != "chess.position" {
        return;
    }

    let state = load_state_for_message(message, api);
    if state == () {
        return;
    }

    if state["channel"] != channel {
        return;
    }

    let action = message["action"];
    if action == "reset" {
        let next_state = initial_state(state["match_id"], state["channel"], state["ticket_key"], state["player_ids"]);
        api.put(state_key(next_state["match_id"]), next_state);
        let reply = build_position_message(next_state, "reset", event.connection_id, message["clientName"], (), "");
        let reply_channel = next_state["channel"];
        api.broadcast_to_channels(reply, [reply_channel]);
        return;
    }

    if action != "move" {
        let reply = build_position_message(state, "rejected", event.connection_id, message["clientName"], message["move"], "unsupported chess action");
        let reply_channel = state["channel"];
        api.broadcast_to_channels(reply, [reply_channel]);
        return;
    }

    let player_color = color_for_player(state, event.connection_id);
    if player_color == "" {
        let reply = build_position_message(state, "rejected", event.connection_id, message["clientName"], message["move"], "player is not part of this match");
        let reply_channel = state["channel"];
        api.broadcast_to_channels(reply, [reply_channel]);
        return;
    }

    if state["status"] != "active" {
        let reply = build_position_message(state, "rejected", event.connection_id, message["clientName"], message["move"], "match is already finished");
        let reply_channel = state["channel"];
        api.broadcast_to_channels(reply, [reply_channel]);
        return;
    }

    if player_color != state["turn"] {
        let reply = build_position_message(state, "rejected", event.connection_id, message["clientName"], message["move"], "not your turn");
        let reply_channel = state["channel"];
        api.broadcast_to_channels(reply, [reply_channel]);
        return;
    }

    let result = apply_move(state, message["move"], player_color);
    if result["ok"] {
        let next_state = result["state"];
        api.put(state_key(next_state["match_id"]), next_state);
        let reply = build_position_message(next_state, "move", event.connection_id, message["clientName"], result["move"], "");
        let reply_channel = next_state["channel"];
        api.broadcast_to_channels(reply, [reply_channel]);
    } else {
        let reply = build_position_message(state, "rejected", event.connection_id, message["clientName"], message["move"], result["error"]);
        let reply_channel = state["channel"];
        api.broadcast_to_channels(reply, [reply_channel]);
    }
}

fn on_match_channel_ready(event, api) {
    let state = initial_state(event.match_id, event.channel, event.ticket_key, event.player_ids);
    api.put(state_key(event.match_id), state);

    let message = #{};
    message["type"] = "match.found";
    message["match_id"] = event.match_id;
    message["channel"] = event.channel;
    message["ticket_key"] = event.ticket_key;
    message["player_ids"] = event.player_ids;
    message["position"] = public_position(state["board"]);
    message["turn"] = state["turn"];
    message["moves"] = state["moves"];

    let channel = event.channel;
    api.broadcast_to_channels(message, [channel]);
}

fn initial_state(match_id, channel, ticket_key, player_ids) {
    let state = #{};
    state["match_id"] = match_id;
    state["channel"] = channel;
    state["ticket_key"] = ticket_key;
    state["player_ids"] = player_ids;
    state["board"] = start_board();
    state["turn"] = "w";
    state["moves"] = 0;
    state["status"] = "active";
    state["winner"] = "";
    state["end_reason"] = "";
    state["en_passant"] = "";
    state["castling"] = castling_rights();
    return state;
}

fn build_position_message(state, action, actor_id, actor_name, move, error) {
    if actor_name == () || actor_name == "" {
        actor_name = actor_id;
    }

    let message = #{};
    message["type"] = "chess.position";
    message["action"] = action;
    message["clientId"] = actor_id;
    message["clientName"] = actor_name;
    message["matchId"] = state["match_id"];
    message["match_id"] = state["match_id"];
    message["channel"] = state["channel"];
    message["player_ids"] = state["player_ids"];
    message["position"] = public_position(state["board"]);
    message["turn"] = state["turn"];
    message["moves"] = state["moves"];
    message["status"] = state["status"];
    message["check"] = is_in_check(state["board"], state["turn"]);
    message["match_over"] = state["status"] != "active";
    message["winner"] = state["winner"];
    message["end_reason"] = state["end_reason"];

    if move != () {
        message["move"] = move;
    }

    if error != "" {
        message["error"] = error;
    }

    return message;
}

fn apply_move(state, move, color) {
    if move == () {
        return rejected("missing move");
    }

    let source = move["source"];
    let target = move["target"];
    let expected_piece = move["piece"];
    let legal = legal_move_result(state, source, target, expected_piece, color);

    if !legal["ok"] {
        return legal;
    }

    let piece = piece_at(state["board"], source);
    let next_state = copy_state(state);
    next_state["board"] = board_after_move(state, source, target, piece, legal);
    update_castling_rights(next_state, source, target, piece);
    update_en_passant(next_state, source, target, piece);
    next_state["moves"] = state["moves"] + 1;
    next_state["turn"] = opponent(color);

    let next_color = next_state["turn"];
    if !has_any_legal_move(next_state, next_color) {
        if is_in_check(next_state["board"], next_color) {
            next_state["status"] = "checkmate";
            next_state["winner"] = color;
            next_state["end_reason"] = "checkmate";
        } else {
            next_state["status"] = "stalemate";
            next_state["winner"] = "";
            next_state["end_reason"] = "stalemate";
        }
    }

    let result = accepted();
    let applied_move = #{};
    applied_move["source"] = source;
    applied_move["target"] = target;
    applied_move["piece"] = piece;
    applied_move["promotion"] = promotion_piece(piece, target);
    result["move"] = applied_move;
    result["state"] = next_state;
    return result;
}

fn legal_move_result(state, source, target, expected_piece, color) {
    if !is_valid_square(source) || !is_valid_square(target) {
        return rejected("invalid square");
    }

    if source == target {
        return rejected("source and target must be different");
    }

    let board = state["board"];
    let piece = piece_at(board, source);
    if piece == "" {
        return rejected("source square is empty");
    }

    if expected_piece != () && expected_piece != "" && expected_piece != piece {
        return rejected("piece does not match source square");
    }

    if piece_color(piece) != color {
        return rejected("cannot move opponent piece");
    }

    let target_piece = piece_at(board, target);
    if target_piece != "" && piece_color(target_piece) == color {
        return rejected("target square contains your own piece");
    }

    let pseudo = pseudo_legal_move(state, source, target, piece);
    if !pseudo["ok"] {
        return pseudo;
    }

    let next_board = board_after_move(state, source, target, piece, pseudo);
    if is_in_check(next_board, color) {
        return rejected("move leaves king in check");
    }

    return pseudo;
}

fn pseudo_legal_move(state, source, target, piece) {
    let board = state["board"];
    let color = piece_color(piece);
    let kind = piece_kind(piece);
    let source_file = file_of(source);
    let source_rank = rank_of(source);
    let target_file = file_of(target);
    let target_rank = rank_of(target);
    let file_delta = target_file - source_file;
    let rank_delta = target_rank - source_rank;
    let target_piece = piece_at(board, target);

    if kind == "P" {
        let direction = pawn_direction(color);
        let start_rank = pawn_start_rank(color);

        if file_delta == 0 && rank_delta == direction && target_piece == "" {
            return accepted_with_kind("move");
        }

        if file_delta == 0 && source_rank == start_rank && rank_delta == 2 * direction && target_piece == "" {
            let middle = square(source_file, source_rank + direction);
            if piece_at(board, middle) == "" {
                return accepted_with_kind("double_pawn");
            }
        }

        if abs(file_delta) == 1 && rank_delta == direction {
            if target_piece != "" && piece_color(target_piece) == opponent(color) {
                return accepted_with_kind("capture");
            }

            if target == state["en_passant"] && target_piece == "" {
                let captured_rank = target_rank - direction;
                let captured_square = square(target_file, captured_rank);
                let captured_piece = opponent(color) + "P";
                let found_piece = piece_at(board, captured_square);
                if found_piece == captured_piece {
                    return accepted_with_kind("en_passant");
                }
            }
        }

        return rejected("illegal pawn move");
    }

    if kind == "N" {
        if (abs(file_delta) == 1 && abs(rank_delta) == 2) || (abs(file_delta) == 2 && abs(rank_delta) == 1) {
            return accepted_with_kind("move");
        }
        return rejected("illegal knight move");
    }

    if kind == "B" {
        if abs(file_delta) == abs(rank_delta) && clear_path(board, source_file, source_rank, target_file, target_rank) {
            return accepted_with_kind("move");
        }
        return rejected("illegal bishop move");
    }

    if kind == "R" {
        if (file_delta == 0 || rank_delta == 0) && clear_path(board, source_file, source_rank, target_file, target_rank) {
            return accepted_with_kind("move");
        }
        return rejected("illegal rook move");
    }

    if kind == "Q" {
        if ((file_delta == 0 || rank_delta == 0) || abs(file_delta) == abs(rank_delta)) && clear_path(board, source_file, source_rank, target_file, target_rank) {
            return accepted_with_kind("move");
        }
        return rejected("illegal queen move");
    }

    if kind == "K" {
        if abs(file_delta) <= 1 && abs(rank_delta) <= 1 {
            return accepted_with_kind("move");
        }

        if rank_delta == 0 && abs(file_delta) == 2 && can_castle(state, color, source, target) {
            return accepted_with_kind("castle");
        }

        return rejected("illegal king move");
    }

    return rejected("unknown chess piece");
}

fn board_after_move(state, source, target, piece, details) {
    let board = copy_board(state["board"]);
    let color = piece_color(piece);
    let kind = piece_kind(piece);

    board[source] = "";

    if details["kind"] == "en_passant" {
        let direction = pawn_direction(color);
        let captured_square = square(file_of(target), rank_of(target) - direction);
        board[captured_square] = "";
    }

    if details["kind"] == "castle" {
        if target == "g1" {
            board["h1"] = "";
            board["f1"] = "wR";
        } else if target == "c1" {
            board["a1"] = "";
            board["d1"] = "wR";
        } else if target == "g8" {
            board["h8"] = "";
            board["f8"] = "bR";
        } else if target == "c8" {
            board["a8"] = "";
            board["d8"] = "bR";
        }
    }

    board[target] = promotion_piece(piece, target);
    return board;
}

fn promotion_piece(piece, target) {
    if piece_kind(piece) == "P" && rank_of(target) == promotion_rank(piece_color(piece)) {
        return piece_color(piece) + "Q";
    }

    return piece;
}

fn update_en_passant(state, source, target, piece) {
    state["en_passant"] = "";

    if piece_kind(piece) == "P" && abs(rank_of(target) - rank_of(source)) == 2 {
        state["en_passant"] = square(file_of(source), rank_of(source) + pawn_direction(piece_color(piece)));
    }
}

fn update_castling_rights(state, source, target, piece) {
    let rights = state["castling"];
    let color = piece_color(piece);

    if piece_kind(piece) == "K" {
        rights[color + "K"] = false;
        rights[color + "Q"] = false;
    }

    if source == "h1" || target == "h1" {
        rights["wK"] = false;
    }
    if source == "a1" || target == "a1" {
        rights["wQ"] = false;
    }
    if source == "h8" || target == "h8" {
        rights["bK"] = false;
    }
    if source == "a8" || target == "a8" {
        rights["bQ"] = false;
    }

    state["castling"] = rights;
}

fn can_castle(state, color, source, target) {
    let board = state["board"];
    let rights = state["castling"];

    if color == "w" && source == "e1" && target == "g1" && rights["wK"] {
        return piece_at(board, "h1") == "wR"
            && piece_at(board, "f1") == ""
            && piece_at(board, "g1") == ""
            && !is_in_check(board, "w")
            && !is_square_attacked(board, "f1", "b")
            && !is_square_attacked(board, "g1", "b");
    }

    if color == "w" && source == "e1" && target == "c1" && rights["wQ"] {
        return piece_at(board, "a1") == "wR"
            && piece_at(board, "b1") == ""
            && piece_at(board, "c1") == ""
            && piece_at(board, "d1") == ""
            && !is_in_check(board, "w")
            && !is_square_attacked(board, "d1", "b")
            && !is_square_attacked(board, "c1", "b");
    }

    if color == "b" && source == "e8" && target == "g8" && rights["bK"] {
        return piece_at(board, "h8") == "bR"
            && piece_at(board, "f8") == ""
            && piece_at(board, "g8") == ""
            && !is_in_check(board, "b")
            && !is_square_attacked(board, "f8", "w")
            && !is_square_attacked(board, "g8", "w");
    }

    if color == "b" && source == "e8" && target == "c8" && rights["bQ"] {
        return piece_at(board, "a8") == "bR"
            && piece_at(board, "b8") == ""
            && piece_at(board, "c8") == ""
            && piece_at(board, "d8") == ""
            && !is_in_check(board, "b")
            && !is_square_attacked(board, "d8", "w")
            && !is_square_attacked(board, "c8", "w");
    }

    return false;
}

fn has_any_legal_move(state, color) {
    let board = state["board"];

    for source in squares() {
        let piece = piece_at(board, source);
        if piece_color(piece) == color {
            for target in squares() {
                if legal_move_result(state, source, target, piece, color)["ok"] {
                    return true;
                }
            }
        }
    }

    return false;
}

fn is_in_check(board, color) {
    let king_square = "";
    for candidate in squares() {
        if piece_at(board, candidate) == color + "K" {
            king_square = candidate;
        }
    }

    if king_square == "" {
        return false;
    }

    return is_square_attacked(board, king_square, opponent(color));
}

fn is_square_attacked(board, target, attacking_color) {
    for source in squares() {
        let piece = piece_at(board, source);
        if piece_color(piece) == attacking_color && piece_attacks_square(board, source, target, piece) {
            return true;
        }
    }

    return false;
}

fn piece_attacks_square(board, source, target, piece) {
    if source == target {
        return false;
    }

    let color = piece_color(piece);
    let kind = piece_kind(piece);
    let source_file = file_of(source);
    let source_rank = rank_of(source);
    let target_file = file_of(target);
    let target_rank = rank_of(target);
    let file_delta = target_file - source_file;
    let rank_delta = target_rank - source_rank;

    if kind == "P" {
        return abs(file_delta) == 1 && rank_delta == pawn_direction(color);
    }

    if kind == "N" {
        return (abs(file_delta) == 1 && abs(rank_delta) == 2) || (abs(file_delta) == 2 && abs(rank_delta) == 1);
    }

    if kind == "B" {
        return abs(file_delta) == abs(rank_delta) && clear_path(board, source_file, source_rank, target_file, target_rank);
    }

    if kind == "R" {
        return (file_delta == 0 || rank_delta == 0) && clear_path(board, source_file, source_rank, target_file, target_rank);
    }

    if kind == "Q" {
        return ((file_delta == 0 || rank_delta == 0) || abs(file_delta) == abs(rank_delta)) && clear_path(board, source_file, source_rank, target_file, target_rank);
    }

    if kind == "K" {
        return abs(file_delta) <= 1 && abs(rank_delta) <= 1;
    }

    return false;
}

fn clear_path(board, source_file, source_rank, target_file, target_rank) {
    let file_step = 0;
    if target_file > source_file {
        file_step = 1;
    } else if target_file < source_file {
        file_step = -1;
    }

    let rank_step = 0;
    if target_rank > source_rank {
        rank_step = 1;
    } else if target_rank < source_rank {
        rank_step = -1;
    }

    let file = source_file + file_step;
    let rank = source_rank + rank_step;

    while file != target_file || rank != target_rank {
        if piece_at(board, square(file, rank)) != "" {
            return false;
        }

        file += file_step;
        rank += rank_step;
    }

    return true;
}

fn start_board() {
    let board = empty_board();
    let pieces = ["R", "N", "B", "Q", "K", "B", "N", "R"];
    let file_list = files();
    let index = 0;

    for piece in pieces {
        board[file_list[index] + "1"] = "w" + piece;
        board[file_list[index] + "2"] = "wP";
        board[file_list[index] + "7"] = "bP";
        board[file_list[index] + "8"] = "b" + piece;
        index += 1;
    }

    return board;
}

fn empty_board() {
    let board = #{};
    for file in files() {
        for rank in ranks() {
            board[file + rank] = "";
        }
    }

    return board;
}

fn public_position(board) {
    let position = #{};
    for square in squares() {
        let piece = piece_at(board, square);
        if piece != "" {
            position[square] = piece;
        }
    }

    return position;
}

fn copy_state(state) {
    let next = #{};
    next["match_id"] = state["match_id"];
    next["channel"] = state["channel"];
    next["ticket_key"] = state["ticket_key"];
    next["player_ids"] = state["player_ids"];
    next["board"] = copy_board(state["board"]);
    next["turn"] = state["turn"];
    next["moves"] = state["moves"];
    next["status"] = state["status"];
    next["winner"] = state["winner"];
    next["end_reason"] = state["end_reason"];
    next["en_passant"] = state["en_passant"];
    next["castling"] = copy_castling(state["castling"]);
    return next;
}

fn copy_board(board) {
    let next = empty_board();
    for square in squares() {
        next[square] = board[square];
    }
    return next;
}

fn copy_castling(rights) {
    let next = #{};
    next["wK"] = rights["wK"];
    next["wQ"] = rights["wQ"];
    next["bK"] = rights["bK"];
    next["bQ"] = rights["bQ"];
    return next;
}

fn castling_rights() {
    let rights = #{};
    rights["wK"] = true;
    rights["wQ"] = true;
    rights["bK"] = true;
    rights["bQ"] = true;
    return rights;
}

fn load_state_for_message(message, api) {
    let match_id = message_match_id(message);
    if match_id == "" {
        return ();
    }

    return api.get(state_key(match_id));
}

fn message_match_id(message) {
    let match_id = message["matchId"];
    if match_id != () && match_id != "" {
        return match_id;
    }

    let snake_match_id = message["match_id"];
    if snake_match_id != () {
        return snake_match_id;
    }

    return "";
}

fn state_key(match_id) {
    return "chess:" + match_id;
}

fn match_channel(channels) {
    for channel in channels {
        if channel != "default" {
            return channel;
        }
    }

    return "";
}

fn color_for_player(state, player_id) {
    let index = 0;
    for candidate in state["player_ids"] {
        if candidate == player_id {
            if index == 0 {
                return "w";
            }
            return "b";
        }
        index += 1;
    }

    return "";
}

fn piece_at(board, square) {
    if !is_valid_square(square) {
        return "";
    }

    return board[square];
}

fn piece_color(piece) {
    if piece == "wP" || piece == "wN" || piece == "wB" || piece == "wR" || piece == "wQ" || piece == "wK" {
        return "w";
    }

    if piece == "bP" || piece == "bN" || piece == "bB" || piece == "bR" || piece == "bQ" || piece == "bK" {
        return "b";
    }

    return "";
}

fn piece_kind(piece) {
    if piece == "wP" || piece == "bP" {
        return "P";
    }
    if piece == "wN" || piece == "bN" {
        return "N";
    }
    if piece == "wB" || piece == "bB" {
        return "B";
    }
    if piece == "wR" || piece == "bR" {
        return "R";
    }
    if piece == "wQ" || piece == "bQ" {
        return "Q";
    }
    if piece == "wK" || piece == "bK" {
        return "K";
    }

    return "";
}

fn opponent(color) {
    if color == "w" {
        return "b";
    }
    return "w";
}

fn pawn_direction(color) {
    if color == "w" {
        return 1;
    }
    return -1;
}

fn pawn_start_rank(color) {
    if color == "w" {
        return 2;
    }
    return 7;
}

fn promotion_rank(color) {
    if color == "w" {
        return 8;
    }
    return 1;
}

fn is_valid_square(square) {
    return file_of(square) != 0 && rank_of(square) != 0;
}

fn square(file_index, rank_index) {
    if file_index < 1 || file_index > 8 || rank_index < 1 || rank_index > 8 {
        return "";
    }

    return files()[file_index - 1] + ranks()[rank_index - 1];
}

fn file_of(square) {
    if square == "a1" { return 1; }
    if square == "a2" { return 1; }
    if square == "a3" { return 1; }
    if square == "a4" { return 1; }
    if square == "a5" { return 1; }
    if square == "a6" { return 1; }
    if square == "a7" { return 1; }
    if square == "a8" { return 1; }
    if square == "b1" { return 2; }
    if square == "b2" { return 2; }
    if square == "b3" { return 2; }
    if square == "b4" { return 2; }
    if square == "b5" { return 2; }
    if square == "b6" { return 2; }
    if square == "b7" { return 2; }
    if square == "b8" { return 2; }
    if square == "c1" { return 3; }
    if square == "c2" { return 3; }
    if square == "c3" { return 3; }
    if square == "c4" { return 3; }
    if square == "c5" { return 3; }
    if square == "c6" { return 3; }
    if square == "c7" { return 3; }
    if square == "c8" { return 3; }
    if square == "d1" { return 4; }
    if square == "d2" { return 4; }
    if square == "d3" { return 4; }
    if square == "d4" { return 4; }
    if square == "d5" { return 4; }
    if square == "d6" { return 4; }
    if square == "d7" { return 4; }
    if square == "d8" { return 4; }
    if square == "e1" { return 5; }
    if square == "e2" { return 5; }
    if square == "e3" { return 5; }
    if square == "e4" { return 5; }
    if square == "e5" { return 5; }
    if square == "e6" { return 5; }
    if square == "e7" { return 5; }
    if square == "e8" { return 5; }
    if square == "f1" { return 6; }
    if square == "f2" { return 6; }
    if square == "f3" { return 6; }
    if square == "f4" { return 6; }
    if square == "f5" { return 6; }
    if square == "f6" { return 6; }
    if square == "f7" { return 6; }
    if square == "f8" { return 6; }
    if square == "g1" { return 7; }
    if square == "g2" { return 7; }
    if square == "g3" { return 7; }
    if square == "g4" { return 7; }
    if square == "g5" { return 7; }
    if square == "g6" { return 7; }
    if square == "g7" { return 7; }
    if square == "g8" { return 7; }
    if square == "h1" { return 8; }
    if square == "h2" { return 8; }
    if square == "h3" { return 8; }
    if square == "h4" { return 8; }
    if square == "h5" { return 8; }
    if square == "h6" { return 8; }
    if square == "h7" { return 8; }
    if square == "h8" { return 8; }
    return 0;
}

fn rank_of(square) {
    if square == "a1" { return 1; }
    if square == "b1" { return 1; }
    if square == "c1" { return 1; }
    if square == "d1" { return 1; }
    if square == "e1" { return 1; }
    if square == "f1" { return 1; }
    if square == "g1" { return 1; }
    if square == "h1" { return 1; }
    if square == "a2" { return 2; }
    if square == "b2" { return 2; }
    if square == "c2" { return 2; }
    if square == "d2" { return 2; }
    if square == "e2" { return 2; }
    if square == "f2" { return 2; }
    if square == "g2" { return 2; }
    if square == "h2" { return 2; }
    if square == "a3" { return 3; }
    if square == "b3" { return 3; }
    if square == "c3" { return 3; }
    if square == "d3" { return 3; }
    if square == "e3" { return 3; }
    if square == "f3" { return 3; }
    if square == "g3" { return 3; }
    if square == "h3" { return 3; }
    if square == "a4" { return 4; }
    if square == "b4" { return 4; }
    if square == "c4" { return 4; }
    if square == "d4" { return 4; }
    if square == "e4" { return 4; }
    if square == "f4" { return 4; }
    if square == "g4" { return 4; }
    if square == "h4" { return 4; }
    if square == "a5" { return 5; }
    if square == "b5" { return 5; }
    if square == "c5" { return 5; }
    if square == "d5" { return 5; }
    if square == "e5" { return 5; }
    if square == "f5" { return 5; }
    if square == "g5" { return 5; }
    if square == "h5" { return 5; }
    if square == "a6" { return 6; }
    if square == "b6" { return 6; }
    if square == "c6" { return 6; }
    if square == "d6" { return 6; }
    if square == "e6" { return 6; }
    if square == "f6" { return 6; }
    if square == "g6" { return 6; }
    if square == "h6" { return 6; }
    if square == "a7" { return 7; }
    if square == "b7" { return 7; }
    if square == "c7" { return 7; }
    if square == "d7" { return 7; }
    if square == "e7" { return 7; }
    if square == "f7" { return 7; }
    if square == "g7" { return 7; }
    if square == "h7" { return 7; }
    if square == "a8" { return 8; }
    if square == "b8" { return 8; }
    if square == "c8" { return 8; }
    if square == "d8" { return 8; }
    if square == "e8" { return 8; }
    if square == "f8" { return 8; }
    if square == "g8" { return 8; }
    if square == "h8" { return 8; }
    return 0;
}

fn squares() {
    return [
        "a1", "a2", "a3", "a4", "a5", "a6", "a7", "a8",
        "b1", "b2", "b3", "b4", "b5", "b6", "b7", "b8",
        "c1", "c2", "c3", "c4", "c5", "c6", "c7", "c8",
        "d1", "d2", "d3", "d4", "d5", "d6", "d7", "d8",
        "e1", "e2", "e3", "e4", "e5", "e6", "e7", "e8",
        "f1", "f2", "f3", "f4", "f5", "f6", "f7", "f8",
        "g1", "g2", "g3", "g4", "g5", "g6", "g7", "g8",
        "h1", "h2", "h3", "h4", "h5", "h6", "h7", "h8"
    ];
}

fn files() {
    return ["a", "b", "c", "d", "e", "f", "g", "h"];
}

fn ranks() {
    return ["1", "2", "3", "4", "5", "6", "7", "8"];
}

fn abs(value) {
    if value < 0 {
        return 0 - value;
    }

    return value;
}

fn accepted() {
    let result = #{};
    result["ok"] = true;
    result["kind"] = "move";
    return result;
}

fn accepted_with_kind(kind) {
    let result = accepted();
    result["kind"] = kind;
    return result;
}

fn rejected(error) {
    let result = #{};
    result["ok"] = false;
    result["error"] = error;
    return result;
}
