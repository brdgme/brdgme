import { ICommandSpec } from "./command";

export interface IGame {
  id: string;
  created_at: string;
  updated_at: string;
  game_version_id: string;
  is_finished: boolean;
}

export interface IGameType {
  id: string;
  created_at: string;
  updated_at: string;
  name: string;
}

export interface IGameVersion {
  id: string;
  created_at: string;
  updated_at: string;
  game_type_id: string;
  name: string;
  is_public: boolean;
  is_deprecated: boolean;
}

export interface IGamePlayer {
  id: string;
  created_at: string;
  updated_at: string;
  user_id: string;
  game_id: string;
  position: number;
  color: string;
  has_accepted: boolean;
  is_turn: boolean;
  is_read: boolean;
  place?: number;
  rating_change?: number;
}

export interface IUser {
  id: string;
  created_at: string;
  updated_at: string;
  name: string;
  pref_colors: string[];
}

export interface IGamePlayerUser {
  game_player: IGamePlayer;
  user: IUser;
}

export interface IGameExtended {
  game: IGame;
  game_type: IGameType;
  game_version: IGameVersion;
  game_players: IGamePlayerUser[];
  game_logs?: IGameLogHTML[];
  pub_state?: string;
  html?: string;
  command_spec?: string;
  chat: IChatExtended[];
}

export interface IGameVersionType {
  game_version: IGameVersion;
  game_type: IGameType;
}

export interface ISession {
  email: string;
  userId: string;
  token: string;
  logout: () => void;
}

export interface IGameLog {
  id: string;
  created_at: string;
  updated_at: string;
  logged_at: string;
  game_id: string;
  is_public: boolean;
  body: string;
}

export interface IGameLogHTML {
  game_log: IGameLog;
  html: string;
}

export interface IChat {
  id: string;
  created_at: string;
  updated_at: string;
}

export interface IChatUser {
  id: string;
  created_at: string;
  updated_at: string;
  chat_id: string;
  user_id: string;
  last_read_at: string;
}

export interface IChatMessage {
  id: string;
  created_at: string;
  updated_at: string;
  chat_user_id: string;
  message: string;
}

export interface IChatExtended {
  chat: IChat;
  chat_users: IChatUser[];
  chat_messages: IChatMessage[];
}
