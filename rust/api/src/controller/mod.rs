use anyhow::Context;
use rocket::http::RawStr;
use rocket::request::{FromParam, Request};
use rocket::response::{self, Responder};
use rocket::{get, options};
use rocket_contrib::json::Json;
use serde::Serialize;
use uuid::Uuid;

use std::path::PathBuf;
use std::str::FromStr;

pub mod auth;
pub mod game;
pub mod mail;

use crate::db::{models, query, CONN};
use crate::errors::ControllerError;

pub struct UuidParam(Uuid);

impl UuidParam {
    pub fn into_uuid(self) -> Uuid {
        self.0
    }
}

impl<'a> FromParam<'a> for UuidParam {
    type Error = ControllerError;

    fn from_param(param: &'a RawStr) -> Result<Self, ControllerError> {
        Ok(UuidParam(
            Uuid::from_str(param).context("failed to parse UUID")?,
        ))
    }
}

pub struct CORS<R>(R);

impl<'r, R: Responder<'r>> Responder<'r> for CORS<R> {
    fn respond_to(self, request: &Request) -> response::Result<'r> {
        let mut response = self.0.respond_to(request)?;
        response.set_raw_header("Access-Control-Allow-Origin", "*");
        response.set_raw_header(
            "Access-Control-Allow-Methods",
            "GET, POST, PUT, DELETE, OPTIONS",
        );
        response.set_raw_header(
            "Access-Control-Allow-Headers",
            "Authorization, Content-Type",
        );
        response.set_raw_header("Access-Control-Allow-Credentials", "true");
        Ok(response)
    }
}

#[options("/<path..>")]
pub fn options(path: PathBuf) -> CORS<()> {
    CORS(())
}

#[derive(Serialize, Debug)]
pub struct InitResponse {
    pub game_version_types: Vec<models::PublicGameVersionType>,
    pub games: Vec<query::PublicGameExtended>,
    pub user: Option<models::PublicUser>,
}

#[get("/init")]
pub fn init(user: Option<models::User>) -> Result<CORS<Json<InitResponse>>, ControllerError> {
    let conn = &*CONN.r.get().context("unable to get connection")?;

    Ok(CORS(Json(InitResponse {
        game_version_types: query::public_game_versions(conn)
            .context("unable to get public game versions")?
            .into_iter()
            .map(|gvt| gvt.into_public())
            .collect(),
        games: user
            .as_ref()
            .map(|u| {
                query::find_active_games_for_user(&u.id, conn)
                    .unwrap()
                    .into_iter()
                    .map(|ge| {
                        user.as_ref()
                            .map(|u| ge.clone().into_public_for_user(&u.id))
                            .unwrap_or_else(|| ge.into_public())
                    })
                    .collect()
            })
            .unwrap_or_else(|| vec![]),
        user: user.map(|u| u.into_public()),
    })))
}
