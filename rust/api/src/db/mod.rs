use anyhow::{anyhow, Result};
use lazy_static::lazy_static;

pub mod color;
pub mod models;
pub mod query;
pub mod schema;

//use r2d2;
//use r2d2_diesel::ConnectionManager;
use diesel::pg::PgConnection;

use crate::config;

lazy_static! {
    pub static ref CONN: Connections = Connections {
        w: Connection {},
        r: Connection {},
    };
}

pub struct Connection {}

impl Connection {
    pub fn get(&self) -> Result<Box<PgConnection>> {
        use diesel::Connection;
        Ok(Box::new(PgConnection::establish(
            &config::PostgresConfig::from_env("")?
                .ok_or_else(|| anyhow!("could not get database config"))?
                .url(),
        )?))
    }
}

pub struct Connections {
    pub w: Connection,
    pub r: Connection,
}

/*lazy_static! {
    pub static ref CONN: Connections = connect_env().unwrap();
}

pub struct Connections {
    pub w: r2d2::Pool<ConnectionManager<PgConnection>>,
    pub r: r2d2::Pool<ConnectionManager<PgConnection>>,
}

pub fn connect(w_addr: &str, r_addr: &str) -> Result<Connections, Error> {
    Ok(Connections {
           w: conn(w_addr)?,
           r: conn(r_addr)?,
       })
}

pub fn connect_env() -> Result<Connections, Error> {
    let w_addr = env::var("DATABASE_URL")
        .context("DATABASE_URL not set")?;
    connect(&w_addr,
            &env::var("DATABASE_URL_R").unwrap_or_else(|_| w_addr.to_owned()))
}

fn conn(addr: &str)
        -> Result<r2d2::Pool<ConnectionManager<PgConnection>>, Error> {
    r2d2::Pool::new(r2d2::Config::default(),
                    ConnectionManager::<PgConnection>::new(addr))
            .context("unable to connect to database")
}
*/
