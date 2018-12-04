import * as Immutable from "immutable";
import { eventChannel } from "redux-saga";
import { call, Effect, fork, put, take, takeEvery, takeLatest } from "redux-saga/effects";

import * as http from "../http";
import * as Records from "../records";
import * as App from "../reducers";
import * as Game from "../reducers/game";
import * as Login from "../reducers/pages/login";
import * as Session from "../reducers/session";

export const LS_AUTH_TOKEN_OFFSET = "token";

export function* sagas(): IterableIterator<Effect> {
  yield takeEvery(Login.SUBMIT_CODE_SUCCESS, loginSuccess);
  yield takeEvery(Session.UPDATE_PATH, updatePath);
  yield takeEvery(Session.UPDATE_TOKEN, updateToken);
  yield takeEvery(Session.CLEAR_TOKEN, clearToken);
  yield fork(hashchange);
}

function* loginSuccess(action: Login.ISubmitCodeSuccess): IterableIterator<Effect> {
  yield put(Session.updateToken(action.payload));
  yield put(Session.updatePath("/"));
}

function* updatePath(action: Session.IUpdatePath): IterableIterator<Effect> {
  yield put(App.clearPageState());
  window.location.hash = action.payload;
}

function* updateToken(action: Session.IUpdateToken): IterableIterator<Effect> {
  localStorage.setItem(LS_AUTH_TOKEN_OFFSET, action.payload);
  const init: http.IInitResponse = yield call(http.fetchInit, action.payload);
  yield put(Session.updateUser(Records.User.fromJS(init.user)));
  yield put(Session.updateGameVersionTypes(
    Immutable.List(
      init.game_version_types.map(Records.GameVersionType.fromJS),
    ),
  ));
  yield put(Game.updateGames(Records.GameExtended.fromJSList(init.games)));
}

function* clearToken(action: Session.IClearToken): IterableIterator<Effect> {
  localStorage.removeItem(LS_AUTH_TOKEN_OFFSET);
  location.reload(true);
}

function* hashchange(): IterableIterator<Effect> {
  const chan = yield call(hashchangeChannel);
  while (true) {
    const newPath = yield take(chan);
    yield put(Session.updatePath(newPath));
  }
}

export function hashchangeChannel() {
  return eventChannel((emitter) => {
    const listener = () => emitter(location.hash.substr(1) || "/");
    window.addEventListener("hashchange", listener);
    window.addEventListener("popstate", listener);
    return () => {
      window.removeEventListener("hashchange", listener);
      window.removeEventListener("popstate", listener);
    };
  });
}
