import * as Immutable from "immutable";
import * as superagent from "superagent";

import * as Model from "./model";

export async function submitLoginEmail(email: string, apiServer: string): Promise<{}> {
  return superagent
    .post(`${apiServer}/auth`)
    .set("Content-Type", "application/json")
    .set("Accept", "application/json")
    .send({ email });
}

export async function submitLoginCode(email: string, code: string, apiServer: string): Promise<string> {
  return superagent
    .post(`${apiServer}/auth/confirm`)
    .set("Content-Type", "application/json")
    .set("Accept", "application/json")
    .send({ email, code })
    .then((res) => res.body as string);
}

export async function fetchActiveGames(apiServer: string, token: string): Promise<Model.IGameExtended[]> {
  return superagent
    .get(`${apiServer}/game/my_active`)
    .auth(token, "")
    .set("Content-Type", "application/json")
    .set("Accept", "application/json")
    .then((res) => res.body.games as Model.IGameExtended[]);
}

export async function fetchGame(id: string, apiServer: string, token?: string): Promise<Model.IGameExtended> {
  return superagent
    .get(`${apiServer}/game/${id}`)
    .auth(token || "", "")
    .set("Content-Type", "application/json")
    .set("Accept", "application/json")
    .then((res) => res.body as Model.IGameExtended);
}

export async function submitGameCommand(id: string, command: string, apiServer: string, token: string): Promise<Model.IGameExtended> {
  return superagent
    .post(`${apiServer}/game/${id}/command`)
    .auth(token, "")
    .set("Content-Type", "application/json")
    .set("Accept", "application/json")
    .send({ command })
    .then((res) => res.body as Model.IGameExtended);
}

export async function submitMarkGameRead(id: string, apiServer: string, token: string): Promise<Model.IGamePlayer> {
  return superagent
    .post(`${apiServer}/game/${id}/mark_read`)
    .auth(token, "")
    .set("Content-Type", "application/json")
    .set("Accept", "application/json")
    .send()
    .then((res) => res.body as Model.IGamePlayer);
}

export async function submitGameConcede(id: string, apiServer: string, token: string): Promise<Model.IGameExtended> {
  return superagent
    .post(`${apiServer}/game/${id}/concede`)
    .auth(token, "")
    .set("Content-Type", "application/json")
    .set("Accept", "application/json")
    .send()
    .then((res) => res.body as Model.IGameExtended);
}

export async function submitUndo(id: string, apiServer: string, token: string): Promise<Model.IGameExtended> {
  return superagent
    .post(`${apiServer}/game/${id}/undo`)
    .auth(token, "")
    .set("Content-Type", "application/json")
    .set("Accept", "application/json")
    .send()
    .then((res) => res.body as Model.IGameExtended);
}

export async function submitRestart(id: string, apiServer: string, token: string): Promise<Model.IGameExtended> {
  return superagent
    .post(`${apiServer}/game/${id}/restart`)
    .auth(token, "")
    .set("Content-Type", "application/json")
    .set("Accept", "application/json")
    .send()
    .then((res) => res.body as Model.IGameExtended);
}

export interface IInitResponse {
  game_version_types: Model.IGameVersionType[];
  games: Model.IGameExtended[];
  user?: Model.IUser;
}

export async function fetchInit(apiServer: string, token?: string): Promise<IInitResponse> {
  let req = superagent
    .get(`${apiServer}/init`)
    .set("Content-Type", "application/json")
    .set("Accept", "application/json");
  if (token !== undefined) {
    req = req.auth(token, "");
  }
  return req.then((res) => res.body as IInitResponse);
}

export async function submitNewGame(
  gameVersionId: string,
  userIds: Immutable.List<string>,
  emails: Immutable.List<string>,
  apiServer: string,
  token: string,
): Promise<Model.IGameExtended> {
  return superagent
    .post(`${apiServer}/game`)
    .auth(token, "")
    .set("Content-Type", "application/json")
    .set("Accept", "application/json")
    .send({
      game_version_id: gameVersionId,
      opponent_emails: emails.toJS(),
      opponent_ids: userIds.toJS(),
    })
    .then((res) => res.body as Model.IGameExtended);
}
