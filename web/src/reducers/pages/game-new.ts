import * as Immutable from "immutable";

import * as Records from "../../records";
import * as Game from "../game";

export enum Mode {
  FetchingGameVersionTypes,
  Editable,
  Submitting,
}

export class State extends Immutable.Record({
  mode: Mode.Editable,
  game_version_types: Immutable.List<Records.GameVersionType>(),
  game_version_id: "",
  emails: Immutable.List<string>(),
  user_ids: Immutable.List<string>(),
}) { }

export const UPDATE_MODE = "brdgme/pages/game-new/UPDATE_MODE";
export const FETCH_GAME_VERSION_TYPES = "brdgme/pages/game-new/FETCH_GAME_VERSION_TYPES";
export const UPDATE_GAME_VERSION_ID = "brdgme/pages/game-new/UPDATE_GAME_VERSION_ID";
export const ADD_EMAIL = "brdgme/pages/game-new/ADD_EMAIL";
export const REMOVE_EMAIL = "brdgme/pages/game-new/REMOVE_EMAIL";
export const UPDATE_EMAIL = "brdgme/pages/game-new/UPDATE_EMAIL";
export const ADD_USER_ID = "brdgme/pages/game-new/ADD_USER_ID";
export const REMOVE_USER_ID = "brdgme/pages/game-new/REMOVE_USER_ID";
export const UPDATE_USER_ID = "brdgme/pages/game-new/UPDATE_USER_ID";
export const SUBMIT = "brdgme/pages/game-new/SUBMIT";
export const SUBMIT_SUCCESS = "brdgme/pages/game-new/SUBMIT_SUCCESS";
export const SUBMIT_FAIL = "brdgme/pages/game-new/SUBMIT_FAIL";

export interface IUpdateMode {
  type: typeof UPDATE_MODE;
  payload: Mode;
}
export const updateMode = (mode: Mode): IUpdateMode => ({
  type: UPDATE_MODE,
  payload: mode,
});

export interface IFetchGameVersionTypes { type: typeof FETCH_GAME_VERSION_TYPES; }
export const fetchGameVersionTypes = (): IFetchGameVersionTypes => ({
  type: FETCH_GAME_VERSION_TYPES,
});

export interface IUpdateGameVersionId {
  type: typeof UPDATE_GAME_VERSION_ID;
  payload: string;
}
export const updateGameVersionId = (game_version_id: string): IUpdateGameVersionId => ({
  type: UPDATE_GAME_VERSION_ID,
  payload: game_version_id,
});

export interface IAddEmail { type: typeof ADD_EMAIL; }
export const addEmail = (): IAddEmail => ({ type: ADD_EMAIL });

export interface IRemoveEmail {
  type: typeof REMOVE_EMAIL;
  payload: number;
}
export const removeEmail = (index: number): IRemoveEmail => ({
  type: REMOVE_EMAIL,
  payload: index,
});

export interface IUpdateEmail {
  type: typeof UPDATE_EMAIL;
  payload: {
    index: number;
    email: string;
  };
}
export const updateEmail = (index: number, email: string): IUpdateEmail => ({
  type: UPDATE_EMAIL,
  payload: { index, email },
});

export interface IAddUserId { type: typeof ADD_USER_ID; }
export const addUserId = (): IAddUserId => ({ type: ADD_USER_ID });

export interface IRemoveUserId {
  type: typeof REMOVE_USER_ID;
  payload: number;
}
export const removeUserId = (index: number): IRemoveUserId => ({
  type: REMOVE_USER_ID,
  payload: index,
});

export interface IUpdateUserId {
  type: typeof UPDATE_USER_ID;
  payload: {
    index: number;
    user_id: string;
  };
}
export const updateUserId = (index: number, user_id: string): IUpdateUserId => ({
  type: UPDATE_USER_ID,
  payload: { index, user_id },
});

export interface ISubmit {
  type: typeof SUBMIT;
  payload: {
    game_version_id: string,
    emails: Immutable.List<string>,
    user_ids: Immutable.List<string>,
  };
}
export const submit = (
  game_version_id: string,
  emails: Immutable.List<string>,
  user_ids: Immutable.List<string>,
) => ({
  type: SUBMIT,
  payload: { game_version_id, emails, user_ids },
});

export interface ISubmitSuccess {
  type: typeof SUBMIT_SUCCESS;
  payload: Records.GameExtended;
}
export const submitSuccess = (game: Records.GameExtended): ISubmitSuccess => ({
  type: SUBMIT_SUCCESS,
  payload: game,
});

export interface ISubmitFail {
  type: typeof SUBMIT_FAIL;
  payload: string;
}
export const submitFail = (error: string): ISubmitFail => ({
  type: SUBMIT_FAIL,
  payload: error,
});

export type Action
  = IUpdateMode
  | IFetchGameVersionTypes
  | IUpdateGameVersionId
  | IAddEmail
  | IRemoveEmail
  | IUpdateEmail
  | IAddUserId
  | IRemoveUserId
  | IUpdateUserId
  | ISubmit
  | ISubmitSuccess
  | ISubmitFail
  ;

export function reducer(state = new State(), action: Action): State {
  switch (action.type) {
    case UPDATE_MODE: return state.set("mode", action.payload);
    case UPDATE_GAME_VERSION_ID: return state.set(
      "game_version_id",
      action.payload,
    );
    case ADD_EMAIL: return state.update(
      "emails",
      (emails) => emails.push(""),
    );
    case REMOVE_EMAIL: return state.update(
      "emails",
      (emails) => emails.delete(action.payload),
    );
    case UPDATE_EMAIL: return state.update(
      "emails",
      (emails) => emails.set(action.payload.index, action.payload.email),
    );
    case ADD_USER_ID: return state.update(
      "user_ids",
      (user_ids) => user_ids.push(""),
    );
    case REMOVE_USER_ID: return state.update(
      "user_ids",
      (user_ids) => user_ids.delete(action.payload),
    );
    case UPDATE_USER_ID: return state.update(
      "user_ids",
      (user_ids) => user_ids.set(action.payload.index, action.payload.user_id),
    );
    case SUBMIT_FAIL:
      alert(`Failed to create game: ${action.payload}`);
      return state;
    default: return state;
  }
}
