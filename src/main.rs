#[macro_use]
extern crate diesel;
extern crate dotenv;
extern crate serde;
extern crate serde_json;

use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use dotenv::dotenv;
use std::env;

pub mod schema;
pub mod models;

use models::*;

pub fn establish_connection() -> SqliteConnection {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");
    SqliteConnection::establish(&database_url)
        .expect(&format!("Error connecting to {}", database_url))
}

fn main() {
    use schema::events::dsl::*;

    let connection = establish_connection();
    let last = events.order(when.desc()).first::<Event>(&connection).expect("Error loading event");
    println!("{}", last.id);
    println!("{}", serde_json::to_string(&last).expect("Error serializing to JSON"));

    let history = events.order(when).load::<Event>(&connection).expect("Error loading history");
    println!("{}", serde_json::to_string(&history).expect("Error serializing history to JSON"));
}
