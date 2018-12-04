import { END, eventChannel } from "redux-saga";
import {
  actionChannel,
  call,
  cancel,
  Effect,
  fork,
  put,
  race,
  select,
  take,
  takeEvery,
  takeLatest,
} from "redux-saga/effects";

import * as http from "../http";
import * as Model from "../model";
import * as Records from "../records";
import { State as AppState } from "../reducers";
import * as Game from "../reducers/game";
import * as Session from "../reducers/session";
import * as WS from "../reducers/ws";

export const LS_AUTH_TOKEN_OFFSET = "token";

interface IMessage {
  GameUpdate?: Model.IGameExtended;
  GameRestarted?: {
    game_id: string;
    restarted_game_id: string;
  };
}

async function sleep(ms: number): Promise<{}> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

export function* sagas(): IterableIterator<Effect> {
  yield fork(wsSaga);
  yield takeEvery(Session.UPDATE_TOKEN, updateToken);
  yield takeEvery(Session.CLEAR_TOKEN, clearToken);
}

export function* updateToken(action: Session.IUpdateToken): IterableIterator<Effect> {
  yield put(WS.subscribeUser(action.payload));
}

export function* clearToken(action: Session.IClearToken): IterableIterator<Effect> {
  yield put(WS.unsubscribeUser());
}

export function* wsSaga(): IterableIterator<Effect> {
  const actions = yield actionChannel([
    WS.SUBSCRIBE_GAME,
    WS.UNSUBSCRIBE_GAME,
    WS.SUBSCRIBE_USER,
    WS.UNSUBSCRIBE_USER,
  ]);
  while (true) {
    try {
      // yield put(WS.connecting());
      const socket: WebSocket = yield call(connect, process.env.WS_SERVER);
      const socketClose = call(socketClosePromise, socket);
      // yield put(WS.connected());
      const s = yield fork(socketSagas, socket);
      while (true) {
        const { action } = yield race({
          action: take(actions),
          close: socketClose,
        });
        if (action) {
          sendAction(socket, action);
        } else {
          break;
        }
      }
    } finally {
      yield put(WS.waitingForReconnect(5));
      yield call(sleep, 5 * 1000);
    }
  }
}

function socketClosePromise(socket: WebSocket): Promise<{}> {
  return new Promise((resolve, reject) =>
    socket.addEventListener("close", () => resolve()));
}

export function* handleMessages(socket: WebSocket): IterableIterator<Effect> {
  const chan = yield call(messageChannel, socket);
  while (true) {
    const message: MessageEvent = yield take(chan);
    const data: IMessage = JSON.parse(message.data);
    if (data.GameUpdate) {
      yield put(Game.updateGames(Records.GameExtended.fromJSList([
        data.GameUpdate,
      ])));
    } else if (data.GameRestarted) {
      yield put(Game.gameRestarted(
        data.GameRestarted.game_id,
        data.GameRestarted.restarted_game_id,
      ));
    }
  }
}

export function messageChannel(socket: WebSocket) {
  return eventChannel((emitter) => {
    const listener = (event: MessageEvent) => {
      emitter(event);
    };
    socket.addEventListener("message", listener);
    socket.addEventListener("close", () => emitter(END));
    return () => {
      socket.removeEventListener("message", listener);
    };
  });
}

function connect(addr: string): Promise<WebSocket> {
  return new Promise((resolve, reject) => {
    const socket = new WebSocket(addr);
    socket.addEventListener("open", (event) => resolve(socket));
    socket.addEventListener("error", (event) => reject(event));
  });
}

function* socketSagas(socket: WebSocket): IterableIterator<Effect> {
  yield fork(handleMessages, socket);
}

function sendAction(socket: WebSocket, action: WS.Action) {
  socket.send(JSON.stringify(action));
}
