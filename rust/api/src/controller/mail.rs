use anyhow::Context;
use rocket::{post, Data};

use std::io::Read;

use crate::{errors::ControllerError, mail::handle_inbound_email};

#[post("/", data = "<data>")]
pub fn index(data: Data) -> Result<(), ControllerError> {
    let mut buffer = String::new();
    data.open()
        .read_to_string(&mut buffer)
        .context("failed to read body")?;
    handle_inbound_email(&buffer);
    Ok(())
}
