import * as Immutable from "immutable";

import * as Model from "../model";
import * as Records from "../records";

export class State extends Immutable.Record({
  games: Immutable.Map<string, Records.GameExtended>(),
}) {
  public updateGames(newGames: Immutable.List<Records.GameExtended>): this {
    return this.set("games", this.games.withMutations((games) => {
      newGames.forEach((g) => {
        if (g === undefined) {
          return;
        }
        let existing: Records.GameExtended | undefined = games.get(g.game.id);
        if (existing !== undefined) {
          if (existing !== undefined && existing.game.updated_at > g.game.updated_at) {
            // Our internal one is already newer.
            return;
          }
          if (existing.game_player && !g.game_player) {
            // We have a private view of the game, don't override with public.
            return;
          }
        } else {
          existing = new Records.GameExtended();
        }
        games.set(g.game.id, existing.withMutations((ex: Records.GameExtended) => {
          const existingLogs = ex.game_logs || Immutable.List<Records.GameLogRendered>();
          ex.merge(g).set("game_logs", existingLogs.withMutations((logs) => {
            const logIds = logs.map((l: Records.GameLogRendered) => l.game_log.id).toSet();
            if (g.game_logs !== undefined) {
              g.game_logs.forEach((newLog: Records.GameLogRendered) => {
                if (!logIds.contains(newLog.game_log.id)) {
                  logs.push(newLog);
                }
              });
            }
          }));
        }));
      });
    }));
  }

  public updateGamePlayer(gamePlayer: Model.IGamePlayer): this {
    if (!this.games.has(gamePlayer.game_id)) {
      return this;
    }
    return this.updateIn(
      ["games", gamePlayer.game_id],
      (game: Records.GameExtended) => game.withMutations((g: Records.GameExtended) => {
        if (g.game_player !== undefined && g.game_player.id === gamePlayer.id) {
          g.update(
            "game_player",
            (gp) => gp && gp.merge(gamePlayer) || Records.GamePlayer.fromJS(gamePlayer));
        }
        g.update("game_players", (gps) => gps.map((gpu) => {
          if (gpu.game_player.id === gamePlayer.id) {
            return gpu.update(
              "game_player",
              (gp) => gp.merge(gamePlayer),
            );
          }
          return gpu;
        }).toList());
      }));
  }
}

export const FETCH_GAME = "brdgme/game/FETCH_GAME";
export const FETCH_GAME_SUCCESS = "brdgme/game/FETCH_GAME_SUCCESS";
export const FETCH_GAME_FAIL = "brdgme/game/FETCH_GAME_FAIL";
export const UPDATE_GAMES = "brdgme/game/UPDATE_GAMES";
export const SUBMIT_COMMAND = "brdgme/game/SUBMIT_COMMAND";
export const SUBMIT_COMMAND_SUCCESS = "brdgme/game/SUBMIT_COMMAND_SUCCESS";
export const SUBMIT_COMMAND_FAIL = "brdgme/game/SUBMIT_COMMAND_FAIL";
export const SUBMIT_UNDO = "brdgme/game/SUBMIT_UNDO";
export const SUBMIT_UNDO_SUCCESS = "brdgme/game/SUBMIT_UNDO_SUCCESS";
export const SUBMIT_UNDO_FAIL = "brdgme/game/SUBMIT_UNDO_FAIL";
export const SUBMIT_MARK_READ = "brdgme/game/SUBMIT_MARK_READ";
export const SUBMIT_MARK_READ_SUCCESS = "brdgme/game/SUBMIT_MARK_READ_SUCCESS";
export const SUBMIT_MARK_READ_FAIL = "brdgme/game/SUBMIT_MARK_READ_FAIL";
export const SUBMIT_CONCEDE = "brdgme/game/SUBMIT_CONCEDE";
export const SUBMIT_CONCEDE_SUCCESS = "brdgme/game/SUBMIT_CONCEDE_SUCCESS";
export const SUBMIT_CONCEDE_FAIL = "brdgme/game/SUBMIT_CONCEDE_FAIL";
export const SUBMIT_RESTART = "brdgme/game/SUBMIT_RESTART";
export const SUBMIT_RESTART_SUCCESS = "brdgme/game/SUBMIT_RESTART_SUCCESS";
export const SUBMIT_RESTART_FAIL = "brdgme/game/SUBMIT_RESTART_FAIL";
export const GAME_RESTARTED = "brdgme/game/GAME_RESTARTED";

export interface IFetchGame {
  type: typeof FETCH_GAME;
  payload: string;
}
export const fetchGame = (id: string): IFetchGame => ({
  type: FETCH_GAME,
  payload: id,
});

export interface IFetchGameSuccess {
  type: typeof FETCH_GAME_SUCCESS;
  payload: Model.IGameExtended;
}
export const fetchGameSuccess =
  (game: Model.IGameExtended): IFetchGameSuccess => ({
    type: FETCH_GAME_SUCCESS,
    payload: game,
  });

export interface IFetchGameFail {
  type: typeof FETCH_GAME_FAIL;
}
export const fetchGameFail = (): IFetchGameFail => ({ type: FETCH_GAME_FAIL });

export interface IUpdateGames {
  type: typeof UPDATE_GAMES;
  payload: Immutable.List<Records.GameExtended>;
}
export const updateGames =
  (games: Immutable.List<Records.GameExtended>): IUpdateGames => ({
    type: UPDATE_GAMES,
    payload: games,
  });

export interface ISubmitCommand {
  type: typeof SUBMIT_COMMAND;
  payload: {
    gameId: string;
    command: string;
  };
}
export const submitCommand =
  (gameId: string, command: string): ISubmitCommand => ({
    type: SUBMIT_COMMAND,
    payload: { gameId, command },
  });

export interface ISubmitCommandSuccess {
  type: typeof SUBMIT_COMMAND_SUCCESS;
  payload: Records.GameExtended;
}
export const submitCommandSuccess =
  (game: Records.GameExtended): ISubmitCommandSuccess => ({
    type: SUBMIT_COMMAND_SUCCESS,
    payload: game,
  });

export interface ISubmitCommandFail {
  type: typeof SUBMIT_COMMAND_FAIL;
  payload: string;
}
export const submitCommandFail = (error: string): ISubmitCommandFail => ({
  type: SUBMIT_COMMAND_FAIL,
  payload: error,
});

export interface ISubmitUndo {
  type: typeof SUBMIT_UNDO;
  payload: string;
}
export const submitUndo =
  (gameId: string): ISubmitUndo => ({
    type: SUBMIT_UNDO,
    payload: gameId,
  });

export interface ISubmitUndoSuccess {
  type: typeof SUBMIT_UNDO_SUCCESS;
  payload: Records.GameExtended;
}
export const submitUndoSuccess =
  (game: Records.GameExtended): ISubmitUndoSuccess => ({
    type: SUBMIT_UNDO_SUCCESS,
    payload: game,
  });

export interface ISubmitUndoFail {
  type: typeof SUBMIT_UNDO_FAIL;
  payload: string;
}
export const submitUndoFail = (error: string): ISubmitUndoFail => ({
  type: SUBMIT_UNDO_FAIL,
  payload: error,
});

export interface ISubmitMarkRead {
  type: typeof SUBMIT_MARK_READ;
  payload: string;
}
export const submitMarkRead =
  (gameId: string): ISubmitMarkRead => ({
    type: SUBMIT_MARK_READ,
    payload: gameId,
  });

export interface ISubmitMarkReadSuccess {
  type: typeof SUBMIT_MARK_READ_SUCCESS;
  payload: Model.IGamePlayer;
}
export const submitMarkReadSuccess =
  (gamePlayer: Model.IGamePlayer): ISubmitMarkReadSuccess => ({
    type: SUBMIT_MARK_READ_SUCCESS,
    payload: gamePlayer,
  });

export interface ISubmitMarkReadFail {
  type: typeof SUBMIT_MARK_READ_FAIL;
  payload: string;
}
export const submitMarkReadFail = (error: string): ISubmitMarkReadFail => ({
  type: SUBMIT_MARK_READ_FAIL,
  payload: error,
});

export interface ISubmitConcede {
  type: typeof SUBMIT_CONCEDE;
  payload: string;
}
export const submitConcede =
  (gameId: string): ISubmitConcede => ({
    type: SUBMIT_CONCEDE,
    payload: gameId,
  });

export interface ISubmitConcedeSuccess {
  type: typeof SUBMIT_CONCEDE_SUCCESS;
  payload: Model.IGameExtended;
}
export const submitConcedeSuccess =
  (game: Model.IGameExtended): ISubmitConcedeSuccess => ({
    type: SUBMIT_CONCEDE_SUCCESS,
    payload: game,
  });

export interface ISubmitConcedeFail {
  type: typeof SUBMIT_CONCEDE_FAIL;
  payload: string;
}
export const submitConcedeFail = (error: string): ISubmitConcedeFail => ({
  type: SUBMIT_CONCEDE_FAIL,
  payload: error,
});

export interface ISubmitRestart {
  type: typeof SUBMIT_RESTART;
  payload: string;
}
export const submitRestart =
  (gameId: string): ISubmitRestart => ({
    type: SUBMIT_RESTART,
    payload: gameId,
  });

export interface ISubmitRestartSuccess {
  type: typeof SUBMIT_RESTART_SUCCESS;
  payload: {
    oldGameId: string;
    newGame: Model.IGameExtended;
  };
}
export const submitRestartSuccess = (
  oldGameId: string,
  newGame: Model.IGameExtended,
): ISubmitRestartSuccess => ({
  type: SUBMIT_RESTART_SUCCESS,
  payload: { oldGameId, newGame },
});

export interface ISubmitRestartFail {
  type: typeof SUBMIT_RESTART_FAIL;
  payload: string;
}
export const submitRestartFail = (error: string): ISubmitRestartFail => ({
  type: SUBMIT_RESTART_FAIL,
  payload: error,
});

export interface IGameRestarted {
  type: typeof GAME_RESTARTED;
  payload: {
    gameId: string;
    restartedGameId: string;
  };
}
export const gameRestarted = (
  gameId: string,
  restartedGameId: string,
): IGameRestarted => ({
  type: GAME_RESTARTED,
  payload: { gameId, restartedGameId },
});

export type Action
  = IFetchGame
  | IFetchGameSuccess
  | IFetchGameFail
  | IUpdateGames
  | ISubmitCommand
  | ISubmitCommandSuccess
  | ISubmitCommandFail
  | ISubmitUndo
  | ISubmitUndoSuccess
  | ISubmitUndoFail
  | ISubmitMarkRead
  | ISubmitMarkReadSuccess
  | ISubmitMarkReadFail
  | ISubmitConcede
  | ISubmitConcedeSuccess
  | ISubmitConcedeFail
  | ISubmitRestart
  | ISubmitRestartSuccess
  | ISubmitRestartFail
  | IGameRestarted
  ;

export function reducer(state = new State(), action: Action): State {
  switch (action.type) {
    case FETCH_GAME_SUCCESS: return state.updateGames(
      Records.GameExtended.fromJSList([action.payload]));
    case UPDATE_GAMES: return state.updateGames(action.payload);
    case SUBMIT_COMMAND_SUCCESS: return state.updateGames(
      Records.GameExtended.fromJSList([action.payload]));
    case SUBMIT_MARK_READ_SUCCESS: return state.updateGamePlayer(action.payload);
    case SUBMIT_CONCEDE_SUCCESS: return state.updateGames(
      Records.GameExtended.fromJSList([action.payload]));
    case SUBMIT_RESTART_SUCCESS: return state
      .updateGames(Records.GameExtended.fromJSList([action.payload.newGame]))
      .updateIn(
        ["games", action.payload.oldGameId, "game"],
        (game: Records.Game) => game.set(
          "restarted_game_id",
          action.payload.newGame.game.id,
        ),
      );
    case GAME_RESTARTED: return state.updateIn(
      ["games", action.payload.gameId, "game"],
      (game: Records.Game) => game.set("restarted_game_id", action.payload.restartedGameId),
    );
    default: return state;
  }
}
