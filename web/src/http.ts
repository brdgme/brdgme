import * as Immutable from "immutable";
import * as superagent from "superagent";

import * as Model from "./model";

export async function submitLoginEmail(email: string): Promise<{}> {
  return superagent
    .post(`/api/auth`)
    .set("Content-Type", "application/json")
    .set("Accept", "application/json")
    .send({ email });
}

export async function submitLoginCode(
  email: string,
  code: string,
): Promise<string> {
  return superagent
    .post(`/api/auth/confirm`)
    .set("Content-Type", "application/json")
    .set("Accept", "application/json")
    .send({ email, code })
    .then((res) => res.body as string);
}

export async function fetchActiveGames(
  token: string,
): Promise<Model.IGameExtended[]> {
  return superagent
    .get(`/api/game/my_active`)
    .set("Authorization", `Bearer ${token}`)
    .set("Content-Type", "application/json")
    .set("Accept", "application/json")
    .then((res) => res.body.games as Model.IGameExtended[]);
}

export async function fetchGame(
  id: string,
  token?: string,
): Promise<Model.IGameExtended> {
  return superagent
    .get(`/api/game/${id}`)
    .set("Authorization", `Bearer ${token}`)
    .set("Content-Type", "application/json")
    .set("Accept", "application/json")
    .then((res) => res.body as Model.IGameExtended);
}

export async function submitGameCommand(
  id: string,
  command: string,
  token: string,
): Promise<Model.IGameExtended> {
  return superagent
    .post(`/api/game/${id}/command`)
    .set("Authorization", `Bearer ${token}`)
    .set("Content-Type", "application/json")
    .set("Accept", "application/json")
    .send({ command })
    .then((res) => res.body as Model.IGameExtended);
}

export async function submitMarkGameRead(
  id: string,
  token: string,
): Promise<Model.IGamePlayer> {
  return superagent
    .post(`/api/game/${id}/mark_read`)
    .set("Authorization", `Bearer ${token}`)
    .set("Content-Type", "application/json")
    .set("Accept", "application/json")
    .send()
    .then((res) => res.body as Model.IGamePlayer);
}

export async function submitGameConcede(
  id: string,
  token: string,
): Promise<Model.IGameExtended> {
  return superagent
    .post(`/api/game/${id}/concede`)
    .set("Authorization", `Bearer ${token}`)
    .set("Content-Type", "application/json")
    .set("Accept", "application/json")
    .send()
    .then((res) => res.body as Model.IGameExtended);
}

export async function submitUndo(
  id: string,
  token: string,
): Promise<Model.IGameExtended> {
  return superagent
    .post(`/api/game/${id}/undo`)
    .set("Authorization", `Bearer ${token}`)
    .set("Content-Type", "application/json")
    .set("Accept", "application/json")
    .send()
    .then((res) => res.body as Model.IGameExtended);
}

export async function submitRestart(
  id: string,
  token: string,
): Promise<Model.IGameExtended> {
  return superagent
    .post(`/api/game/${id}/restart`)
    .set("Authorization", `Bearer ${token}`)
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

export async function fetchInit(token?: string): Promise<IInitResponse> {
  let req = superagent
    .get(`/api/init`)
    .set("Content-Type", "application/json")
    .set("Accept", "application/json");
  if (token !== undefined) {
    req.set("Authorization", `Bearer ${token}`);
  }
  return req.then((res) => res.body as IInitResponse);
}

export async function submitNewGame(
  gameVersionId: string,
  userIds: Immutable.List<string>,
  emails: Immutable.List<string>,
  token: string,
): Promise<Model.IGameExtended> {
  return superagent
    .post(`/api/game`)
    .set("Authorization", `Bearer ${token}`)
    .set("Content-Type", "application/json")
    .set("Accept", "application/json")
    .send({
      game_version_id: gameVersionId,
      opponent_emails: emails.toJS(),
      opponent_ids: userIds.toJS(),
    })
    .then((res) => res.body as Model.IGameExtended);
}
