#![recursion_limit = "1024"]
#![allow(dead_code)]
#![allow(unused_variables)]

// We need this for schema.rs to work properly
#[macro_use]
extern crate diesel;

use rocket::routes;

mod config;
mod controller;
mod db;
mod errors;
mod game_client;
mod mail;
mod render;
mod websocket;

use std::sync::Mutex;
use std::thread;

fn main() {
    let (pub_queue, pub_queue_tx) = websocket::PubQueue::new();
    thread::spawn(move || pub_queue.run());

    rocket::ignite()
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
        .mount("/mail", routes![controller::mail::index])
        .mount("/", routes![controller::options, controller::init])
        .launch();
}
