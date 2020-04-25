#![recursion_limit = "1024"]
#![allow(dead_code)]
#![allow(unused_variables)]
#![feature(plugin)]
#![plugin(rocket_codegen)]
#![feature(custom_derive)]

extern crate chrono;
#[macro_use]
extern crate diesel;
extern crate email;
extern crate env_logger;
#[macro_use]
extern crate failure;
extern crate hyper;
extern crate hyper_rustls;
#[macro_use]
extern crate lazy_static;
extern crate lettre;
#[macro_use]
extern crate log;
extern crate r2d2;
extern crate r2d2_diesel;
extern crate rand;
extern crate redis;
extern crate rocket;
extern crate rocket_contrib;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate unicase;
extern crate uuid;

extern crate brdgme_cmd;
extern crate brdgme_color;
extern crate brdgme_game;
extern crate brdgme_markup;

mod config;
mod controller;
mod db;
mod mail;
mod game_client;
mod errors;
mod websocket;
mod render;

use std::thread;
use std::sync::Mutex;

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
