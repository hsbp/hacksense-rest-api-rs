#[macro_use]
extern crate diesel;
extern crate dotenv;
extern crate serde;
extern crate serde_json;
extern crate askama;
extern crate actix_web;

use actix_web::{web, App, HttpResponse, HttpServer, Result};
use askama::Template;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use dotenv::dotenv;
use std::env;
use std::fmt::Write;
use std::collections::HashMap;

pub mod schema;
pub mod models;

const CSV_EVENT_LENGTH: usize = 59;

use models::*;

#[derive(Template)]
#[template(path = "history.xml")]
pub struct HistoryXML {
    history: Vec<Event>,
}

#[derive(Template)]
#[template(path = "history.html")]
pub struct HistoryHTML {
    history: Vec<Event>,
}

#[derive(Template)]
#[template(path = "status.html")]
pub struct Status<'a> {
	open_closed: &'a str,
	when: String,
}

#[derive(Template)]
#[template(path = "home.html")]
pub struct Home;

pub fn establish_connection() -> SqliteConnection {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");
    SqliteConnection::establish(&database_url)
        .expect(&format!("Error connecting to {}", database_url))
}

pub fn get_last_event() -> Event {
    use schema::events::dsl::*;
    let connection = establish_connection();
    events.order(when.desc()).first::<Event>(&connection).unwrap()
}

pub fn get_status() -> Status<'static> {
    let last = get_last_event();
	Status { open_closed: if last.what { "open" } else { "closed" }, when: last.when }
}

pub fn get_history() -> Vec<Event> {
    use schema::events::dsl::*;
    let connection = establish_connection();
    events.order(when).load::<Event>(&connection).unwrap()
}

pub fn event_to_csv(dst: & mut String, src: &Event) -> Result<(), std::fmt::Error> {
    write!(dst, "{};{};{}\n", src.id, src.when, if src.what { '1' } else { '0' })
}

async fn home(_query: web::Query<HashMap<String, String>>) -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().content_type("text/html").body(Home.render().unwrap()))
}

async fn status_json(_query: web::Query<HashMap<String, String>>) -> Result<HttpResponse> {
    let last = get_last_event();
    Ok(HttpResponse::Ok().content_type("application/json").body(serde_json::to_string(&last)?))
}

async fn status_xml(_query: web::Query<HashMap<String, String>>) -> Result<HttpResponse> {
    let last = get_last_event();
    Ok(HttpResponse::Ok().content_type("text/xml").body(last.render().unwrap()))
}

async fn status_csv(_query: web::Query<HashMap<String, String>>) -> Result<HttpResponse> {
    let mut csv = String::with_capacity(CSV_EVENT_LENGTH);
    event_to_csv(&mut csv, &get_last_event());
    Ok(HttpResponse::Ok().content_type("text/csv").body(csv))
}

async fn status_txt(_query: web::Query<HashMap<String, String>>) -> Result<HttpResponse> {
    let tpl = get_status();
    Ok(HttpResponse::Ok().content_type("text/plain").body(format!("H.A.C.K. is currently {} since {}", tpl.open_closed, tpl.when)))
}

async fn status_html(_query: web::Query<HashMap<String, String>>) -> Result<HttpResponse> {
    let tpl = get_status();
    Ok(HttpResponse::Ok().content_type("text/html").body(tpl.render().unwrap()))
}

async fn history_json(_query: web::Query<HashMap<String, String>>) -> Result<HttpResponse> {
    let history = get_history();
    Ok(HttpResponse::Ok().content_type("application/json").body(serde_json::to_string(&history)?))
}

async fn history_xml(_query: web::Query<HashMap<String, String>>) -> Result<HttpResponse> {
    let tpl = HistoryXML { history: get_history() };
    Ok(HttpResponse::Ok().content_type("text/xml").body(tpl.render().unwrap()))
}

async fn history_csv(_query: web::Query<HashMap<String, String>>) -> Result<HttpResponse> {
    let history = get_history();
    let mut csv = String::with_capacity(history.len() * CSV_EVENT_LENGTH);
    for event in history {
        event_to_csv(&mut csv, &event);
    }
    Ok(HttpResponse::Ok().content_type("text/csv").body(csv))
}

async fn history_html(_query: web::Query<HashMap<String, String>>) -> Result<HttpResponse> {
    let tpl = HistoryHTML { history: get_history() };
    Ok(HttpResponse::Ok().content_type("text/html").body(tpl.render().unwrap()))
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    // start http server
    HttpServer::new(move || {
        App::new()
			.service(web::resource("/").route(web::get().to(home)))
			.service(web::resource("/status.json").route(web::get().to(status_json)))
			.service(web::resource("/status.txt").route(web::get().to(status_txt)))
			.service(web::resource("/status.csv").route(web::get().to(status_csv)))
			.service(web::resource("/status.xml").route(web::get().to(status_xml)))
			.service(web::resource("/status").route(web::get().to(status_html)))
			.service(web::resource("/history.json").route(web::get().to(history_json)))
			.service(web::resource("/history.csv").route(web::get().to(history_csv)))
			.service(web::resource("/history.xml").route(web::get().to(history_xml)))
			.service(web::resource("/history").route(web::get().to(history_html)))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
