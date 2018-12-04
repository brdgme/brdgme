import * as Immutable from "immutable";

export enum Mode {
  EnteringEmail,
  SubmittingEmail,
  EnteringCode,
  SubmittingCode,
}

export class State extends Immutable.Record({
  code: "",
  email: "",
  mode: Mode.EnteringEmail,
}) {
  public code: string;
  public email: string;
  public mode: Mode;
}

export const UPDATE_EMAIL = "brdgme/pages/login/UPDATE_EMAIL";
export const UPDATE_CODE = "brdgme/pages/login/UPDATE_CODE";
export const UPDATE_MODE = "brdgme/pages/login/UPDATE_MODE";
export const SUBMIT_EMAIL = "brdgme/pages/login/SUBMIT_EMAIL";
export const SUBMIT_EMAIL_SUCCESS = "brdgme/pages/login/SUBMIT_EMAIL_SUCCESS";
export const SUBMIT_EMAIL_FAIL = "brdgme/pages/login/SUBMIT_EMAIL_FAIL";
export const SUBMIT_CODE = "brdgme/pages/login/SUBMIT_CODE";
export const SUBMIT_CODE_SUCCESS = "brdgme/pages/login/SUBMIT_CODE_SUCCESS";
export const SUBMIT_CODE_FAIL = "brdgme/pages/login/SUBMIT_CODE_FAIL";

export interface IUpdateEmail {
  type: typeof UPDATE_EMAIL;
  payload: string;
}
export const updateEmail = (email: string): IUpdateEmail => ({
  type: UPDATE_EMAIL,
  payload: email,
});

export interface IUpdateCode {
  type: typeof UPDATE_CODE;
  payload: string;
}
export const updateCode = (code: string): IUpdateCode => ({
  type: UPDATE_CODE,
  payload: code,
});

export interface IUpdateMode {
  type: typeof UPDATE_MODE;
  payload: Mode;
}
export const updateMode = (mode: Mode): IUpdateMode => ({
  type: UPDATE_MODE,
  payload: mode,
});

export interface ISubmitEmail {
  type: typeof SUBMIT_EMAIL;
  payload: string;
}
export const submitEmail = (email: string): ISubmitEmail => ({
  type: SUBMIT_EMAIL,
  payload: email,
});

export interface ISubmitEmailSuccess {
  type: typeof SUBMIT_EMAIL_SUCCESS;
}
export const submitEmailSuccess = (): ISubmitEmailSuccess => ({
  type: SUBMIT_EMAIL_SUCCESS,
});

export interface ISubmitEmailFail {
  type: typeof SUBMIT_EMAIL_FAIL;
}
export const submitEmailFail = (): ISubmitEmailFail => ({
  type: SUBMIT_EMAIL_FAIL,
});

export interface ISubmitCode {
  type: typeof SUBMIT_CODE;
  payload: {
    email: string;
    code: string;
  };
}
export const submitCode = (email: string, code: string): ISubmitCode => ({
  type: SUBMIT_CODE,
  payload: { email, code },
});

export interface ISubmitCodeSuccess {
  type: typeof SUBMIT_CODE_SUCCESS;
  payload: string;
}
export const submitCodeSuccess = (token: string): ISubmitCodeSuccess => ({
  type: SUBMIT_CODE_SUCCESS,
  payload: token,
});

export interface ISubmitCodeFail {
  type: typeof SUBMIT_CODE_FAIL;
}
export const submitCodeFail = (): ISubmitCodeFail => ({
  type: SUBMIT_CODE_FAIL,
});

export type Action
  = IUpdateEmail
  | IUpdateCode
  | IUpdateMode
  | ISubmitEmail
  | ISubmitEmailSuccess
  | ISubmitEmailFail
  | ISubmitCode
  | ISubmitCodeSuccess
  | ISubmitCodeFail
  ;

export function reducer(state = new State(), action: Action): State {
  switch (action.type) {
    case UPDATE_EMAIL: return state.set("email", action.payload);
    case UPDATE_CODE: return state.set("code", action.payload);
    case UPDATE_MODE: return state.set("mode", action.payload);
    case SUBMIT_EMAIL: return state.set("mode", Mode.SubmittingEmail);
    case SUBMIT_EMAIL_SUCCESS: return state.set("mode", Mode.EnteringCode);
    case SUBMIT_EMAIL_FAIL: return state.set("mode", Mode.EnteringEmail);
    case SUBMIT_CODE: return state.set("mode", Mode.SubmittingCode);
    case SUBMIT_CODE_FAIL: return state.set("mode", Mode.EnteringCode);
    default: return state;
  }
}
