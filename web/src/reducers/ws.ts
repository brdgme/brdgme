import * as Immutable from "immutable";

export enum ConnectionState {
  Connecting,
  Connected,
  WaitingForReconnect,
}

export class State extends Immutable.Record({
  connectionState: ConnectionState.Connecting,
  secondsBeforeReconnect: undefined as number | undefined,
  subUser: undefined as string | undefined,
  subGame: undefined as string | undefined,
}) { }

export const CONNECTED = "brdgme/ws/CONNECTED";
export const CONNECTING = "brdgme/ws/CONNECTING";
export const WAITING_FOR_RECONNECT = "brdgme/ws/WAITING_FOR_RECONNECT";
export const SUBSCRIBE_USER = "brdgme/ws/SUBSCRIBE_USER";
export const UNSUBSCRIBE_USER = "brdgme/ws/UNSUBSCRIBE_USER";
export const SUBSCRIBE_GAME = "brdgme/ws/SUBSCRIBE_GAME";
export const UNSUBSCRIBE_GAME = "brdgme/ws/UNSUBSCRIBE_GAME";

export interface IConnected { type: typeof CONNECTED; }
export const connected = (): IConnected => ({ type: CONNECTED });

export interface IConnecting { type: typeof CONNECTING; }
export const connecting = (): IConnecting => ({ type: CONNECTING });

export interface IWaitingForReconnect {
  type: typeof WAITING_FOR_RECONNECT;
  payload: number;
}
export const waitingForReconnect = (waitSeconds: number): IWaitingForReconnect => ({
  type: WAITING_FOR_RECONNECT,
  payload: waitSeconds,
});

export interface ISubscribeUser {
  type: typeof SUBSCRIBE_USER;
  payload: string;
}
export const subscribeUser = (token: string): ISubscribeUser => ({
  type: SUBSCRIBE_USER,
  payload: token,
});

export interface IUnsubscribeUser { type: typeof UNSUBSCRIBE_USER; }
export const unsubscribeUser = (): IUnsubscribeUser => ({
  type: UNSUBSCRIBE_USER,
});

export interface ISubscribeGame {
  type: typeof SUBSCRIBE_GAME;
  payload: string;
}
export const subscribeGame = (id: string): ISubscribeGame => ({
  type: SUBSCRIBE_GAME,
  payload: id,
});

export interface IUnsubscribeGame { type: typeof UNSUBSCRIBE_GAME; }
export const unsubscribeGame = (): IUnsubscribeGame => ({
  type: UNSUBSCRIBE_GAME,
});

export type Action
  = IConnected
  | IConnecting
  | IWaitingForReconnect
  | ISubscribeUser
  | IUnsubscribeUser
  | ISubscribeGame
  | IUnsubscribeGame
  ;

export function reducer(state = new State(), action: Action): State {
  switch (action.type) {
    case CONNECTING:
      return state.set("connectionState", ConnectionState.Connecting);
    case CONNECTED:
      return state
        .set("connectionState", ConnectionState.Connected)
        .remove("secondsBeforeReconnect");
    case WAITING_FOR_RECONNECT:
      return state
        .set("connectionState", ConnectionState.WaitingForReconnect)
        .set("secondsBeforeReconnect", action.payload);
    case SUBSCRIBE_USER: return state.set("subUser", action.payload);
    case UNSUBSCRIBE_USER: return state.remove("subUser");
    case SUBSCRIBE_GAME: return state.set("subGame", action.payload);
    case UNSUBSCRIBE_GAME: return state.remove("subGame");
    default: return state;
  }
}
