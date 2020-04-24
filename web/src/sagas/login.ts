import { call, Effect, put, takeEvery, takeLatest, select } from "redux-saga/effects";

import * as http from "../http";
import * as Login from "../reducers/pages/login";
import * as Session from "../reducers/session";
import { State as AppState } from "../reducers";

export function* sagas(): IterableIterator<Effect> {
  yield takeEvery(Login.SUBMIT_EMAIL, submitLoginEmail),
    yield takeEvery(Login.SUBMIT_CODE, submitLoginCode);
}

function* submitLoginEmail(action: Login.ISubmitEmail): IterableIterator<Effect> {
  try {
    yield call(http.submitLoginEmail, action.payload.email, action.payload.apiServer);
    yield put(Login.submitEmailSuccess());
  } catch (e) {
    yield put(Login.submitEmailFail());
  }
}

function* submitLoginCode(action: Login.ISubmitCode): IterableIterator<Effect> {
  try {
    const token: string = yield call(
      http.submitLoginCode,
      action.payload.email,
      action.payload.code,
      action.payload.apiServer,
    );
    yield put(Login.submitCodeSuccess(token, action.payload.apiServer, action.payload.wsServer));
  } catch (e) {
    yield put(Login.submitCodeFail());
  }
}
