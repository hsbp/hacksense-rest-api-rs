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
use std::collections::HashMap;

pub mod schema;
pub mod models;

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

async fn home(_query: web::Query<HashMap<String, String>>) -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().content_type("text/html").body(Home.render().unwrap()))
}

async fn status_json(_query: web::Query<HashMap<String, String>>) -> Result<HttpResponse> {
    use schema::events::dsl::*;
    let connection = establish_connection();
    let last = events.order(when.desc()).first::<Event>(&connection).unwrap();
    Ok(HttpResponse::Ok().content_type("application/json").body(serde_json::to_string(&last)?))
}

async fn status_xml(_query: web::Query<HashMap<String, String>>) -> Result<HttpResponse> {
    use schema::events::dsl::*;
    let connection = establish_connection();
    let last = events.order(when.desc()).first::<Event>(&connection).unwrap();
    Ok(HttpResponse::Ok().content_type("text/xml").body(last.render().unwrap()))
}

async fn status_txt(_query: web::Query<HashMap<String, String>>) -> Result<HttpResponse> {
    use schema::events::dsl::*;
    let connection = establish_connection();
    let last = events.order(when.desc()).first::<Event>(&connection).unwrap();
	let tpl = Status { open_closed: if last.what { "open" } else { "closed" }, when: last.when };
    Ok(HttpResponse::Ok().content_type("text/plain").body(format!("H.A.C.K. is currently {} since {}", tpl.open_closed, tpl.when)))
}

async fn status_html(_query: web::Query<HashMap<String, String>>) -> Result<HttpResponse> {
    use schema::events::dsl::*;
    let connection = establish_connection();
    let last = events.order(when.desc()).first::<Event>(&connection).unwrap();
	let tpl = Status { open_closed: if last.what { "open" } else { "closed" }, when: last.when };
    Ok(HttpResponse::Ok().content_type("text/html").body(tpl.render().unwrap()))
}

async fn history_json(_query: web::Query<HashMap<String, String>>) -> Result<HttpResponse> {
    use schema::events::dsl::*;
    let connection = establish_connection();
    let history = events.order(when).load::<Event>(&connection).unwrap();
    Ok(HttpResponse::Ok().content_type("application/json").body(serde_json::to_string(&history)?))
}

async fn history_xml(_query: web::Query<HashMap<String, String>>) -> Result<HttpResponse> {
    use schema::events::dsl::*;
    let connection = establish_connection();
    let history = events.order(when).load::<Event>(&connection).unwrap();
    let tpl = HistoryXML { history };
    Ok(HttpResponse::Ok().content_type("text/xml").body(tpl.render().unwrap()))
}

async fn history_html(_query: web::Query<HashMap<String, String>>) -> Result<HttpResponse> {
    use schema::events::dsl::*;
    let connection = establish_connection();
    let history = events.order(when).load::<Event>(&connection).unwrap();
    let tpl = HistoryHTML { history };
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
			.service(web::resource("/status.xml").route(web::get().to(status_xml)))
			.service(web::resource("/status").route(web::get().to(status_html)))
			.service(web::resource("/history.json").route(web::get().to(history_json)))
			.service(web::resource("/history.xml").route(web::get().to(history_xml)))
			.service(web::resource("/history").route(web::get().to(history_html)))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
