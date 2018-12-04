import { call, Effect, put, takeEvery, takeLatest } from "redux-saga/effects";

import * as http from "../http";
import * as Login from "../reducers/pages/login";
import * as Session from "../reducers/session";

export function* sagas(): IterableIterator<Effect> {
  yield takeEvery(Login.SUBMIT_EMAIL, submitLoginEmail),
    yield takeEvery(Login.SUBMIT_CODE, submitLoginCode);
}

function* submitLoginEmail(action: Login.ISubmitEmail): IterableIterator<Effect> {
  try {
    yield call(http.submitLoginEmail, action.payload || "");
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
    );
    yield put(Login.submitCodeSuccess(token));
  } catch (e) {
    yield put(Login.submitCodeFail());
  }
}
