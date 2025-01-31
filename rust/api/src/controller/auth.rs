use anyhow::{anyhow, Context, Error, Result};
use rocket::http::Status;
use rocket::post;
use rocket::request::{FromRequest, Outcome, Request};
use rocket::serde::json::Json;
use serde::Deserialize;
use uuid::Uuid;

use crate::controller::CORS;
use crate::db::models::*;
use crate::db::{query, CONN};
use crate::errors::ControllerError;

#[derive(Deserialize)]
pub struct CreateForm {
    email: String,
}

#[post("/", data = "<data>")]
pub fn create(data: Json<CreateForm>) -> Result<CORS<()>, ControllerError> {
    let create_email = data.into_inner().email;
    let conn = &mut *CONN.w.get().context("unable to get connection")?;
    let confirmation =
        query::user_login_request(&create_email, conn).context("unable to request user login")?;

    //     mail::send(
    //         EmailBuilder::new()
    //             .to(create_email.as_ref())
    //             .from("play@brdg.me")
    //             .subject("brdg.me login confirmation")
    //             .html(&mail::html_layout(&format!(
    //                 "Your brdg.me confirmation is <b>{}</b>

    // This confirmation will expire in 30 minutes if not used.",
    //                 confirmation
    //             )))
    //             .build()
    //             .context("unable to create login confirmation email")?
    //             .into(),
    //     )
    //     .context("unable to send login confirmation email")?;

    Ok(CORS(()))
}

#[derive(Deserialize)]
pub struct ConfirmRequest {
    email: String,
    code: String,
}

#[post("/confirm", data = "<data>")]
// pub fn confirm(data: Json<ConfirmRequest>) -> Result<CORS<Json<String>>> {
pub fn confirm(data: Json<ConfirmRequest>) -> Result<CORS<Json<String>>, ControllerError> {
    let data = data.into_inner();
    let conn = &mut *CONN.w.get().context("unable to get connection")?;

    match query::user_login_confirm(&data.email, &data.code, conn)
        .context("unable to confirm login")?
    {
        Some(token) => Ok(CORS(Json(token.id.to_string()))),
        None => Err(anyhow!("unable to confirm login").into()),
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for User {
    type Error = Error;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let auth_header = match req.headers().get_one("Authorization") {
            Some(a) => a,
            None => {
                return Outcome::Error((
                    Status::Unauthorized,
                    anyhow!("missing Authorization header"),
                ))
            }
        };
        if !auth_header.starts_with("Bearer ") {
            return Outcome::Error((
                Status::Unauthorized,
                anyhow!("expected Bearer Authorization header"),
            ));
        }
        let token = match Uuid::parse_str(&auth_header[7..]) {
            Ok(uuid) => uuid,
            Err(_) => {
                return Outcome::Error((
                    Status::Unauthorized,
                    anyhow!("Authorization password not in valid format"),
                ))
            }
        };
        let conn = &mut *match CONN.r.get() {
            Ok(c) => c,
            Err(_) => {
                return Outcome::Error((
                    Status::InternalServerError,
                    anyhow!("error getting connection"),
                ))
            }
        };

        match query::authenticate(&token, conn) {
            Ok(Some(user)) => Outcome::Success(user),
            _ => Outcome::Error((Status::Unauthorized, anyhow!("invalid credentials"))),
        }
    }
}
