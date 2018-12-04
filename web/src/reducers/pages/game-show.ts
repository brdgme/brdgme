import * as Immutable from "immutable";

import * as Command from "../../command";
import * as Game from "../game";

export class State extends Immutable.Record({
  command: "",
  commandPos: 0,
  commandFocused: false,
  submittingCommand: false,
  commandError: undefined as string | undefined,
  suggestions: Immutable.List(),
  allSuggestions: Immutable.List(),
  subMenuOpen: false,
}) { }

export const UPDATE_COMMAND = "brdgme/pages/game-show/UPDATE_COMMAND";
export const TOGGLE_SUB_MENU = "brdgme/pages/game-show/TOGGLE_SUB_MENU";
export const COMMAND_FOCUS = "brdgme/pages/game-show/COMMAND_FOCUS";
export const COMMAND_BLUR = "brdgme/pages/game-show/COMMAND_BLUR";

export interface IUpdateCommand {
  type: typeof UPDATE_COMMAND;
  payload: {
    command: string;
    commandPos: number;
    commandSpec?: Immutable.Map<any, any>,
  };
}
export const updateCommand = (
  command: string,
  commandPos: number,
  commandSpec?: Immutable.Map<any, any>,
): IUpdateCommand => ({
  type: UPDATE_COMMAND,
  payload: { command, commandPos, commandSpec },
});

export interface IToggleSubMenu {
  type: typeof TOGGLE_SUB_MENU;
}
export const toggleSubMenu = (): IToggleSubMenu => ({ type: TOGGLE_SUB_MENU });

export interface ICommandFocus {
  type: typeof COMMAND_FOCUS;
}
export const commandFocus = (): ICommandFocus => ({ type: COMMAND_FOCUS });

export interface ICommandBlur {
  type: typeof COMMAND_BLUR;
}
export const commandBlur = (): ICommandBlur => ({ type: COMMAND_BLUR });

type Action
  = IUpdateCommand
  | IToggleSubMenu
  | ICommandFocus
  | ICommandBlur
  | Game.Action;

export function reducer(state = new State(), action: Action): State {
  switch (action.type) {
    case UPDATE_COMMAND: return state
      .set("command", action.payload.command)
      .set("commandPos", action.payload.commandPos);
    case COMMAND_FOCUS: return state.set("commandFocused", true);
    case COMMAND_BLUR: return state.set("commandFocused", false);
    case TOGGLE_SUB_MENU: return state.update("subMenuOpen", (s) => !s);
    case Game.SUBMIT_COMMAND:
    case Game.SUBMIT_UNDO:
      return state.set("submittingCommand", true);
    case Game.SUBMIT_COMMAND_SUCCESS:
    case Game.SUBMIT_UNDO_SUCCESS:
      return state
        .set("submittingCommand", false)
        .set("command", "")
        .set("commandPos", 0)
        .remove("commandError");
    case Game.SUBMIT_COMMAND_FAIL: return state
      .set("commandError", action.payload)
      .set("submittingCommand", false);
    case Game.SUBMIT_UNDO_FAIL: return state
      .set("submittingCommand", false);
    default: return state;
  }
}
