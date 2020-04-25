use rocket::request::{self, FromRequest, Request};
use rocket_contrib::Json;
use rocket::http::Status;
use rocket::http::hyper::header::Basic;
use rocket::Outcome;
use lettre::email::EmailBuilder;
use uuid::Uuid;
use failure::{Error, ResultExt};

use std::str::FromStr;

use db::{query, CONN};
use db::models::*;
use mail;
use controller::CORS;

#[derive(Deserialize)]
pub struct CreateForm {
    email: String,
}

#[post("/", data = "<data>")]
pub fn create(data: Json<CreateForm>) -> Result<CORS<()>, Error> {
    let create_email = data.into_inner().email;
    let conn = &*CONN.w.get().context("unable to get connection")?;
    let confirmation =
        query::user_login_request(&create_email, conn).context("unable to request user login")?;

    mail::send(EmailBuilder::new()
        .to(create_email.as_ref())
        .from("play@brdg.me")
        .subject("brdg.me login confirmation")
        .html(&mail::html_layout(&format!(
            "Your brdg.me confirmation is <b>{}</b>

This confirmation will expire in 30 minutes if not used.",
            confirmation
        )))
        .build()
        .context("unable to create login confirmation email")?)
        .context({
        "unable to send login confirmation email"
    })?;

    Ok(CORS(()))
}

#[derive(Deserialize)]
pub struct ConfirmRequest {
    email: String,
    code: String,
}

#[post("/confirm", data = "<data>")]
pub fn confirm(data: Json<ConfirmRequest>) -> Result<CORS<Json<String>>, Error> {
    let data = data.into_inner();
    let conn = &*CONN.w.get().context("unable to get connection")?;

    match query::user_login_confirm(&data.email, &data.code, conn)
        .context("unable to confirm login")?
    {
        Some(token) => Ok(CORS(Json(token.id.to_string()))),
        None => Err(format_err!("unable to confirm login")),
    }
}

impl<'a, 'r> FromRequest<'a, 'r> for User {
    type Error = Error;

    fn from_request(request: &'a Request<'r>) -> request::Outcome<Self, Error> {
        let auth_header = match request.headers().get_one("Authorization") {
            Some(a) => a,
            None => {
                return Outcome::Failure((
                    Status::Unauthorized,
                    format_err!("missing Authorization header"),
                ))
            }
        };
        if !auth_header.starts_with("Basic ") {
            return Outcome::Failure((
                Status::Unauthorized,
                format_err!("expected Basic Authorization header"),
            ));
        }
        let auth = match Basic::from_str(&auth_header[6..]) {
            Ok(a) => a,
            Err(e) => {
                return Outcome::Failure((
                    Status::Unauthorized,
                    format_err!("invalid Authorization header"),
                ))
            }
        };
        let token = match Uuid::parse_str(&auth.username) {
            Ok(uuid) => uuid,
            Err(_) => {
                return Outcome::Failure((
                    Status::Unauthorized,
                    format_err!("Authorization password not in valid format"),
                ))
            }
        };
        let conn = &*match CONN.r.get() {
            Ok(c) => c,
            Err(_) => {
                return Outcome::Failure((
                    Status::InternalServerError,
                    format_err!("error getting connection"),
                ))
            }
        };

        match query::authenticate(&token, conn) {
            Ok(Some(user)) => Outcome::Success(user),
            _ => Outcome::Failure((Status::Unauthorized, format_err!("invalid credentials"))),
        }
    }
}
