import * as Immutable from "immutable";
import { call, Effect, put, select, takeEvery, takeLatest } from "redux-saga/effects";

import * as http from "../http";
import * as Model from "../model";
import * as Records from "../records";
import { State as AppState } from "../reducers";
import * as Game from "../reducers/game";
import * as GameNew from "../reducers/pages/game-new";
import * as Session from "../reducers/session";

export function* sagas(): IterableIterator<Effect> {
  yield takeEvery(Game.FETCH_GAME, fetchGame);
  yield takeEvery(Game.SUBMIT_COMMAND, submitCommand);
  yield takeEvery(Game.SUBMIT_UNDO, submitUndo);
  yield takeEvery(Game.SUBMIT_RESTART, submitRestart);
  yield takeEvery(Game.SUBMIT_MARK_READ, submitMarkRead);
  yield takeEvery(Game.SUBMIT_CONCEDE, submitConcede);
  yield takeEvery(GameNew.SUBMIT, submitNewGame);
}

function* fetchGame(action: Game.IFetchGame): IterableIterator<Effect> {
  const token: string = yield select((state: AppState) => state.session.token);
  try {
    const game = yield call(http.fetchGame, action.payload, token);
    yield put(Game.fetchGameSuccess(game));
  } catch (e) {
    yield put(Game.fetchGameFail());
  }
}

function* submitCommand(action: Game.ISubmitCommand): IterableIterator<Effect> {
  const token: string = yield select((state: AppState) => state.session.token);
  if (token === undefined) {
    return;
  }
  try {
    const game = yield call(
      http.submitGameCommand,
      action.payload.gameId,
      action.payload.command,
      token,
    );
    yield put(Game.submitCommandSuccess(game));
  } catch (e) {
    yield put(Game.submitCommandFail(e.response && e.response.text || e.message));
  }
}

function* submitUndo(action: Game.ISubmitUndo): IterableIterator<Effect> {
  const token: string = yield select((state: AppState) => state.session.token);
  if (token === undefined) {
    return;
  }
  try {
    const game = yield call(
      http.submitUndo,
      action.payload,
      token,
    );
    yield put(Game.submitUndoSuccess(game));
  } catch (e) {
    yield put(Game.submitUndoFail(e.response && e.response.text || e.message));
  }
}

function* submitRestart(action: Game.ISubmitRestart): IterableIterator<Effect> {
  const token: string = yield select((state: AppState) => state.session.token);
  if (token === undefined) {
    return;
  }
  try {
    const game: Model.IGameExtended = yield call(
      http.submitRestart,
      action.payload,
      token,
    );
    yield put(Game.submitRestartSuccess(action.payload, game));
    yield put(Session.updatePath(`/game/${game.game.id}`));
  } catch (e) {
    yield put(Game.submitRestartFail(e.response && e.response.text || e.message));
  }
}

function* submitMarkRead(action: Game.ISubmitMarkRead): IterableIterator<Effect> {
  const token: string = yield select((state: AppState) => state.session.token);
  if (token === undefined) {
    return;
  }
  try {
    const gamePlayer = yield call(
      http.submitMarkGameRead,
      action.payload,
      token,
    );
    yield put(Game.submitMarkReadSuccess(gamePlayer));
  } catch (e) {
    yield put(Game.submitMarkReadFail(e.response && e.response.text || e.message));
  }
}

function* submitConcede(action: Game.ISubmitConcede): IterableIterator<Effect> {
  const token: string = yield select((state: AppState) => state.session.token);
  if (token === undefined) {
    return;
  }
  try {
    const game = yield call(
      http.submitGameConcede,
      action.payload,
      token,
    );
    yield put(Game.submitConcedeSuccess(game));
  } catch (e) {
    yield put(Game.submitConcedeFail(e.response && e.response.text || e.message));
  }
}

function* submitNewGame(action: GameNew.ISubmit): IterableIterator<Effect> {
  const token: string = yield select((state: AppState) => state.session.token);
  if (token === undefined) {
    return;
  }
  try {
    const game = Records.GameExtended.fromJS(yield call(
      http.submitNewGame,
      action.payload.game_version_id,
      action.payload.emails,
      action.payload.user_ids,
      token,
    ));
    yield put(Game.updateGames(Immutable.List([game])));
    yield put(Session.updatePath(`/game/${game.game.id}`));
    yield put(GameNew.submitSuccess(game));
  } catch (e) {
    yield put(GameNew.submitFail(e.response && e.response.text || e.message));
  }
}
