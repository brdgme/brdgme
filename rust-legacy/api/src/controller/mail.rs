use mail::handle_inbound_email;
use rocket::Data;
use failure::Error;

use std::io::Read;

#[post("/", data = "<data>")]
pub fn index(data: Data) -> Result<(), Error> {
    let mut buffer = String::new();
    data.open().read_to_string(&mut buffer)?;
    handle_inbound_email(&buffer);
    Ok(())
}
