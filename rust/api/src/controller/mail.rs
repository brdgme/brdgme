use failure::Error;
use rocket::{post, Data};

use std::io::Read;

use crate::mail::handle_inbound_email;

#[post("/", data = "<data>")]
pub fn index(data: Data) -> Result<(), Error> {
    let mut buffer = String::new();
    data.open().read_to_string(&mut buffer)?;
    handle_inbound_email(&buffer);
    Ok(())
}
