import * as Immutable from "immutable";
import { combineReducers } from "redux-immutable";

import * as GameNew from "./game-new";
import * as GameShow from "./game-show";
import * as Login from "./login";

export class State extends Immutable.Record({
  gameNew: new GameNew.State(),
  gameShow: new GameShow.State(),
  login: new Login.State(),
}) {
  public gameNew: GameNew.State;
  public gameShow: GameShow.State;
  public login: Login.State;
}

export const reducer = combineReducers<State>({
  gameNew: GameNew.reducer,
  gameShow: GameShow.reducer,
  login: Login.reducer,
});
