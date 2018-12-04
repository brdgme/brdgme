import * as React from "react";
import * as ReactDOM from "react-dom";
import * as ReactRedux from "react-redux";
import * as Redux from "redux";
import createSagaMiddleware from "redux-saga";

import * as Session from "./reducers/session";
import sagas from "./sagas";
import { LS_AUTH_TOKEN_OFFSET } from "./sagas/session";

import "./style.less"; // tslint:disable-line

import { Container as AppContainer } from "./components/app";
import { reducer as App, State } from "./reducers";

interface IMyWindow extends Window {
  __REDUX_DEVTOOLS_EXTENSION_COMPOSE__: <R>(a: R) => R;
}
const composeEnhancers = window.__REDUX_DEVTOOLS_EXTENSION_COMPOSE__ || Redux.compose;
declare var window: IMyWindow;

// Create store and initialise path and token.
const sagaMiddleware = createSagaMiddleware();
const store = Redux.createStore(
  App,
  new State(),
  composeEnhancers(Redux.applyMiddleware(
    sagaMiddleware,
  )),
);
sagaMiddleware.run(sagas);
store.dispatch(Session.updatePath(location.hash.substr(1)));
const token = localStorage.getItem(LS_AUTH_TOKEN_OFFSET);
if (token !== null) {
  store.dispatch(Session.updateToken(token));
}

ReactDOM.render(
  <ReactRedux.Provider store={store}>
    <AppContainer />
  </ReactRedux.Provider >,
  document.body,
);
