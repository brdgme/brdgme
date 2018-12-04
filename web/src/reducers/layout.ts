import * as Immutable from "immutable";

export class State extends Immutable.Record({
  menuOpen: false,
}) { }

export const TOGGLE_MENU = "brdgme/layout/TOGGLE_MENU";
export const CLOSE_MENU = "brdgme/layout/CLOSE_MENU";

export interface IToggleMenu {
  type: typeof TOGGLE_MENU;
}
export const toggleMenu = (): IToggleMenu => ({
  type: TOGGLE_MENU,
});

export interface ICloseMenu {
  type: typeof CLOSE_MENU;
}
export const closeMenu = (): ICloseMenu => ({
  type: CLOSE_MENU,
});

export type Action
  = IToggleMenu
  | ICloseMenu
  ;

export function reducer(state = new State(), action: Action): State {
  switch (action.type) {
    case TOGGLE_MENU: return state.update("menuOpen", (menuOpen) => !menuOpen);
    case CLOSE_MENU: return state.set("menuOpen", false);
    default: return state;
  }
}
