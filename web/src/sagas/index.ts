import { call, Effect, fork, put, takeEvery, takeLatest } from "redux-saga/effects";

import { sagas as gameSagas } from "./game";
import { sagas as loginSagas } from "./login";
import { sagas as sessionSagas } from "./session";
import { sagas as wsSagas } from "./ws";

export default function* sagas(): IterableIterator<Effect[]> {
  yield [
    fork(gameSagas),
    fork(loginSagas),
    fork(sessionSagas),
    fork(wsSagas),
  ];
}
