#![recursion_limit = "1024"]
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(clippy::large_enum_variant)]
#![allow(clippy::too_many_arguments)]

// We need this for schema.rs to work properly
#[macro_use]
extern crate diesel;

#[macro_use]
extern crate rocket;

use rocket::routes;

mod config;
mod controller;
mod db;
mod errors;
mod game_client;
mod render;
mod websocket;

use std::sync::Mutex;
use std::thread;

#[launch]
fn rocket() -> _ {
    let (pub_queue, pub_queue_tx) = websocket::PubQueue::new().expect("failed to create pub queue");
    thread::spawn(move || pub_queue.run());

    rocket::build()
        .manage(Mutex::new(pub_queue_tx))
        .mount(
            "/game",
            routes![
                controller::game::create,
                controller::game::show,
                controller::game::command,
                controller::game::undo,
                controller::game::mark_read,
                controller::game::concede,
                controller::game::restart,
            ],
        )
        .mount(
            "/auth",
            routes![controller::auth::create, controller::auth::confirm,],
        )
        .mount("/", routes![controller::options, controller::init])
}
