import * as Immutable from "immutable";

import { ICommandSpec } from "./command";

export class User extends Immutable.Record({
  id: "",
  created_at: "",
  updated_at: "",
  name: "",
  pref_colors: Immutable.List<string>(),
}) {
  public static fromJS(js: any): User {
    return new User(js);
  }
}

export class GameType extends Immutable.Record({
  id: "",
  created_at: "",
  updated_at: "",
  name: "",
  player_counts: Immutable.List<number>(),
  weight: 0,
}) {
  public static fromJS(js: any): GameType {
    return new GameType(js);
  }
}

export class GameVersion extends Immutable.Record({
  id: "",
  created_at: "",
  updated_at: "",
  game_type_id: "",
  name: "",
  is_public: false,
  is_deprecated: false,
}) {
  public static fromJS(js: any): GameVersion {
    return new GameVersion(js);
  }
}

export class GameVersionType extends Immutable.Record({
  game_version: new GameVersion(),
  game_type: new GameType(),
}) {
  public static fromJS(js: any): GameVersionType {
    return new GameVersionType({
      game_version: GameVersion.fromJS(js.game_version),
      game_type: GameType.fromJS(js.game_type),
    });
  }
}

export class Game extends Immutable.Record({
  id: "",
  created_at: "",
  updated_at: "",
  game_version_id: "",
  is_finished: false,
  finished_at: undefined as string | undefined,
  chat_id: undefined as string | undefined,
  restarted_game_id: undefined as string | undefined,
}) {
  public static fromJS(js: any): Game {
    return new Game(js);
  }
}

export class GameTypeUser extends Immutable.Record({
  id: "",
  created_at: "",
  updated_at: "",
  game_type_id: "",
  user_id: "",
  rating: 0,
  peak_rating: 0,
}) {
  public static fromJS(js: any): GameTypeUser {
    return new GameTypeUser(js);
  }
}

export class GamePlayer extends Immutable.Record({
  id: "",
  created_at: "",
  updated_at: "",
  game_id: "",
  user_id: "",
  position: 0,
  color: "",
  has_accepted: false,
  is_turn: false,
  is_turn_at: "",
  last_turn_at: "",
  is_eliminated: false,
  is_read: false,
  points: undefined as number | undefined,
  can_undo: false,
  place: undefined as number | undefined,
  rating_change: undefined as number | undefined,
}) {
  public static fromJS(js: any): GamePlayer {
    return new GamePlayer(js);
  }
}

export class GameLog extends Immutable.Record({
  id: "",
  created_at: "",
  updated_at: "",
  game_id: "",
  body: "",
  is_public: false,
  logged_at: "",
}) {
  public static fromJS(js: any): GameLog {
    return new GameLog(js);
  }
}

export class GameLogRendered extends Immutable.Record({
  game_log: new GameLog(),
  html: "",
}) {
  public static fromJS(js: any): GameLogRendered {
    return new GameLogRendered({
      game_log: GameLog.fromJS(js.game_log),
      html: js.html,
    });
  }
}

export class GamePlayerTypeUser extends Immutable.Record({
  game_player: new GamePlayer(),
  user: new User(),
  game_type_user: new GameTypeUser(),
}) {
  public static fromJS(js: any): GamePlayerTypeUser {
    return new GamePlayerTypeUser({
      game_player: GamePlayer.fromJS(js.game_player),
      user: User.fromJS(js.user),
      game_type_user: GameTypeUser.fromJS(js.game_type_user),
    });
  }
}

export class GameExtended extends Immutable.Record({
  game: new Game(),
  game_type: new GameType(),
  game_version: new GameVersion(),
  game_player: undefined as GamePlayer | undefined,
  game_players: Immutable.List<GamePlayerTypeUser>(),
  chat: undefined as ChatExtended | undefined,
  game_logs: undefined as Immutable.List<GameLogRendered> | undefined,
  pub_state: undefined as string | undefined,
  html: undefined as string | undefined,
  command_spec: undefined as Immutable.Map<any, any> | undefined,
}) {
  public static fromJS(js: any): GameExtended {
    return new GameExtended({
      game: Game.fromJS(js.game),
      game_type: GameType.fromJS(js.game_type),
      game_version: GameVersion.fromJS(js.game_version),
      game_player: js.game_player && GamePlayer.fromJS(js.game_player) || undefined,
      game_players: Immutable.List<GamePlayerTypeUser>(js.game_players.map(GamePlayerTypeUser.fromJS)),
      game_logs: js.game_logs && Immutable.List<GameLogRendered>(js.game_logs.map(GameLogRendered.fromJS)) || undefined,
      pub_state: js.pub_state,
      html: js.html,
      command_spec: js.command_spec && Immutable.fromJS(js.command_spec) || undefined,
      chat: js.chat && ChatExtended.fromJS(js.chat) || undefined,
    });
  }

  public static fromJSList(js: any): Immutable.List<GameExtended> {
    return Immutable.List<GameExtended>(js.map(GameExtended.fromJS));
  }
}

export class Chat extends Immutable.Record({
  id: "",
  created_at: "",
  updated_at: "",
}) {
  public static fromJS(js: any): Chat {
    return new Chat(js);
  }
}

export class ChatUser extends Immutable.Record({
  id: "",
  created_at: "",
  updated_at: "",
  chat_id: "",
  user_id: "",
  last_read_at: "",
}) {
  public static fromJS(js: any): ChatUser {
    return new ChatUser(js);
  }
}

export class ChatMessage extends Immutable.Record({
  id: "",
  created_at: "",
  updated_at: "",
  chat_user_id: "",
  message: "",
}) {
  public static fromJS(js: any): ChatMessage {
    return new ChatMessage(js);
  }
}

export class ChatExtended extends Immutable.Record({
  chat: new Chat(),
  chat_users: Immutable.List<ChatUser>(),
  chat_messages: Immutable.List<ChatMessage>(),
}) {
  public static fromJS(js: any): ChatExtended {
    return new ChatExtended({
      chat: Chat.fromJS(js.chat),
      chat_users: Immutable.List<ChatUser>(js.chat_users.map(ChatUser.fromJS)),
      chat_messages: Immutable.List<ChatMessage>(js.chat_messages.map(ChatMessage.fromJS)),
    });
  }
}
