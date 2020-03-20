#[macro_use]
extern crate diesel;
extern crate dotenv;
extern crate serde;
extern crate serde_json;
extern crate askama;
extern crate actix_web;
extern crate hmac;
extern crate sha2;
extern crate chrono;
extern crate mime;

use actix_web::{web, App, HttpMessage, HttpRequest, HttpResponse, HttpServer, Result};
use actix_web::dev::HttpResponseBuilder;
use actix_web::http::header;
use askama::Template;
use chrono::{Local, TimeZone};
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use diesel::r2d2::{self, ConnectionManager};
use dotenv::dotenv;
use hmac::{Hmac, Mac};
use serde_json::json;
use sha2::Sha256;
use std::env;
use std::fmt::Write;
use std::collections::HashMap;

type HmacSha256 = Hmac<Sha256>;
type EventFormatter = fn(Event, &mut HttpResponseBuilder) -> HttpResponse;
type DbPool = r2d2::Pool<ConnectionManager<SqliteConnection>>;

pub mod schema;
pub mod models;

const CSV_EVENT_LENGTH: usize = 59;
static TIMESTAMP_FORMAT: &str = "%Y-%m-%d %H:%M:%S";
static CSV_HEAD: &str = "ID;Timestamp;Status\n";

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
#[template(path = "rss.xml")]
pub struct RSS<'a> {
    title: &'a str,
    id: &'a str,
    pub_date: &'a str,
}

#[derive(Template)]
#[template(path = "home.html")]
pub struct Home;

pub fn get_last_event(pool: web::Data<DbPool>) -> Event {
    use schema::events::dsl::*;
    let connection = pool.get().expect("couldn't get db connection from pool");
    events.order(when.desc()).first::<Event>(&connection).unwrap()
}

pub fn get_history(pool: web::Data<DbPool>) -> Vec<Event> {
    use schema::events::dsl::*;
    let connection = pool.get().expect("couldn't get db connection from pool");
    events.order(when).load::<Event>(&connection).unwrap()
}

pub fn event_to_csv(dst: & mut String, src: &Event) -> Result<(), std::fmt::Error> {
    writeln!(dst, "{};{};{}", src.id, src.when, if src.what { '1' } else { '0' })
}

async fn home(_query: web::Query<HashMap<String, String>>) -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().content_type("text/html").body(Home.render().unwrap()))
}

fn status_json(last: Event, hrb: &mut HttpResponseBuilder) -> HttpResponse {
    hrb.json(last)
}

async fn status_auto(pool: web::Data<DbPool>, req: HttpRequest) -> Result<HttpResponse> {
    if let Some(a) = req.get_header::<header::Accept>().as_deref() {
        for qi in a {
            let i = &qi.item;
            match (i.type_(), i.subtype()) {
                (mime::TEXT, mime::PLAIN) => return format_status_etag(pool, req, status_txt, None),
                (mime::TEXT, mime::CSV) => return format_status_etag(pool, req, status_csv, None),
                (mime::TEXT, mime::XML) => return format_status_etag(pool, req, status_xml, None),
                (mime::TEXT, mime::HTML) => return format_status_etag(pool, req, status_html, None),
                (mime::APPLICATION, mime::JSON) => return format_status_etag(pool, req, status_json, None),
                (mime::APPLICATION, st) => if st == "rss" { return format_status_etag(pool, req, status_rss, None) },
                _ => (),
            }
        }
    }
    format_status_etag(pool, req, status_html, None)
}

async fn format_status_git(pool: web::Data<DbPool>, req: HttpRequest, formatter: EventFormatter) -> Result<HttpResponse> {
    format_status_etag(pool, req, formatter, Some(&include_str!("../.git/refs/heads/master")[..8]))
}

async fn format_status(pool: web::Data<DbPool>, req: HttpRequest, formatter: EventFormatter) -> Result<HttpResponse> {
    format_status_etag(pool, req, formatter, None)
}

fn format_status_etag(pool: web::Data<DbPool>, req: HttpRequest, formatter: EventFormatter, prefix: Option<&str>) -> Result<HttpResponse> {
    let last = get_last_event(pool);
    let etag_payload = match prefix {
        Some(p) => format!("{}-{}", p, &last.id),
        None => last.id.clone(),
    };
    let etag = header::EntityTag::strong(etag_payload);

    let send_reply = match req.get_header::<header::IfNoneMatch>() {
        Some(header::IfNoneMatch::Any) => false,
        Some(header::IfNoneMatch::Items(ref items)) => !items.iter().any(|item| item.strong_eq(&etag)),
        None => true,
    };

    if send_reply {
        let mut hrb = HttpResponse::Ok();
        hrb.set(header::ETag(etag));
        Ok(formatter(last, &mut hrb))
    } else {
        Ok(HttpResponse::NotModified().finish())
    }
}

fn status_xml(last: Event, hrb: &mut HttpResponseBuilder) -> HttpResponse {
    hrb.content_type("text/xml").body(last.render().unwrap())
}

fn status_rss(last: Event, hrb: &mut HttpResponseBuilder) -> HttpResponse {
    let rfc2822 = Local.datetime_from_str(&last.when, TIMESTAMP_FORMAT).unwrap().to_rfc2822();
    let rss = RSS {
        title: &format!("H.A.C.K. has {}", if last.what { "opened" } else { "closed" }),
        id: &last.id,
        pub_date: &rfc2822,
    };
    hrb.content_type("application/rss+xml").body(rss.render().unwrap())
}

fn status_csv(last: Event, hrb: &mut HttpResponseBuilder) -> HttpResponse {
    let mut csv = String::with_capacity(CSV_EVENT_LENGTH);
    event_to_csv(&mut csv, &last);
    hrb.content_type("text/csv").body(csv)
}

fn status_txt(last: Event, hrb: &mut HttpResponseBuilder) -> HttpResponse {
    let tpl = last.get_status();
    hrb.content_type("text/plain").body(format!("H.A.C.K. is currently {} since {}", tpl.open_closed, tpl.when))
}

fn status_html(last: Event, hrb: &mut HttpResponseBuilder) -> HttpResponse {
    let tpl = last.get_status();
    hrb.content_type("text/html").body(tpl.render().unwrap())
}

async fn history_json(pool: web::Data<DbPool>, _query: web::Query<HashMap<String, String>>) -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(get_history(pool)))
}

async fn history_xml(pool: web::Data<DbPool>, _query: web::Query<HashMap<String, String>>) -> Result<HttpResponse> {
    let tpl = HistoryXML { history: get_history(pool) };
    Ok(HttpResponse::Ok().content_type("text/xml").body(tpl.render().unwrap()))
}

async fn history_csv(pool: web::Data<DbPool>, _query: web::Query<HashMap<String, String>>) -> Result<HttpResponse> {
    let history = get_history(pool);
    let mut csv = String::with_capacity(history.len() * CSV_EVENT_LENGTH + CSV_HEAD.len());
    write!(&mut csv, "{}", CSV_HEAD);
    for event in history {
        event_to_csv(&mut csv, &event);
    }
    Ok(HttpResponse::Ok().content_type("text/csv").body(csv))
}

async fn history_html(pool: web::Data<DbPool>, _query: web::Query<HashMap<String, String>>) -> Result<HttpResponse> {
    let tpl = HistoryHTML { history: get_history(pool) };
    Ok(HttpResponse::Ok().content_type("text/html").body(tpl.render().unwrap()))
}

async fn submit(pool: web::Data<DbPool>, path: web::Path<String>) -> HttpResponse {
    let parts: Vec<&str> = path.split('!').collect();
    let (id, status, mac) = match &parts[..] {
        [id, status, mac] => (id, status, mac),
        _ => return HttpResponse::Unauthorized().finish(),
    };
    let event = Submission {
        id,
        what: status.starts_with('1'),
        when: Local::now().format(TIMESTAMP_FORMAT).to_string(),
    };
    let mac_bytes = match hex::decode(mac) {
        Ok(b) => b,
        _ => return HttpResponse::Unauthorized().finish(),
    };
    let subject = format!("{}!{}", id, status);
    let mut mac = HmacSha256::new_varkey(include_bytes!("../hacksense.key")).unwrap();
    mac.input(subject.as_bytes());
    if mac.verify(&mac_bytes).is_ok() {
        use schema::events::dsl::*;
        let connection = pool.get().expect("couldn't get db connection from pool");
        diesel::insert_into(events).values(&event).execute(&connection); // ignore PK violation
        HttpResponse::Ok().content_type("text/plain").body("OK\n")
    } else {
        HttpResponse::Unauthorized().finish()
    }
}

fn status_spaceapi(last: Event, hrb: &mut HttpResponseBuilder) -> HttpResponse {
    let unix_ts = Local.datetime_from_str(&last.when, TIMESTAMP_FORMAT).unwrap().timestamp();
    let status = json!({
        "api": "0.13",
        "contact": {
            "email": "hack@hsbp.org",
            "facebook": "https://www.facebook.com/hackerspace.budapest",
            "irc": "irc://irc.atw-inter.net/hspbp",
            "jabber": "hack@conference.xmpp.hsbp.org",
            "ml": "hspbp@googlegroups.com",
            "phone": "+36 1 445 4225",
            "twitter": "@hackerspacebp"
        },
        "ext_ccc": "chaostreff",
        "feeds": {
            "blog": {
                "type": "rss",
                "url": "https://hsbp.org/tiki-blogs_rss.php?ver=2"
            },
            "calendar": {
                "type": "rss",
                "url": "https://hsbp.org/tiki-calendars_rss.php?ver=2"
            },
            "wiki": {
                "type": "rss",
                "url": "https://hsbp.org/tiki-wiki_rss.php?ver=2"
            }
        },
        "issue_report_channels": ["email"],
        "location": {
            "address": "Bástya u. 12., 1056 Budapest, Hungary",
            "lat": 47.489167,
            "lon": 19.059444
        },
        "logo": "https://hsbp.org/img/hack.gif",
        "projects": [
            "https://github.com/hsbp",
            "https://hsbp.org/projects",
            "https://hsbp.org/hwprojektek"
        ],
        "space": "H.A.C.K.",
        "state": {
            "lastchange": unix_ts,
            "open": last.what
        },
        "url": "https://hsbp.org"
    });
    hrb.json(status)
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");
    let manager = ConnectionManager::<SqliteConnection>::new(database_url);
    let pool = r2d2::Pool::builder()
                .build(manager).expect("Failed to create pool.");
    // start http server
    HttpServer::new(move || {
        App::new()
            .data(pool.clone())
			.service(web::resource("/").route(web::get().to(home)))
			.service(web::resource("/submit/{data}").route(web::get().to(submit)))
			.service(web::resource("/spaceapi_status.json").route(web::get().to(|pool: web::Data<DbPool>, req: HttpRequest| format_status_git(pool, req, status_spaceapi))))
			.service(web::resource("/status.json").route(web::get().to(|pool: web::Data<DbPool>, req: HttpRequest| format_status(pool, req, status_json))))
			.service(web::resource("/status.txt").route(web::get().to(|pool: web::Data<DbPool>, req: HttpRequest| format_status(pool, req, status_txt))))
			.service(web::resource("/status.csv").route(web::get().to(|pool: web::Data<DbPool>, req: HttpRequest| format_status(pool, req, status_csv))))
			.service(web::resource("/status.rss").route(web::get().to(|pool: web::Data<DbPool>, req: HttpRequest| format_status(pool, req, status_rss))))
			.service(web::resource("/status.xml").route(web::get().to(|pool: web::Data<DbPool>, req: HttpRequest| format_status(pool, req, status_xml))))
			.service(web::resource("/status").route(web::get().to(status_auto)))
			.service(web::resource("/history.json").route(web::get().to(history_json)))
			.service(web::resource("/history.csv").route(web::get().to(history_csv)))
			.service(web::resource("/history.xml").route(web::get().to(history_xml)))
			.service(web::resource("/history").route(web::get().to(history_html)))
    })
    .bind("127.0.0.1:8000")?
    .run()
    .await
}
