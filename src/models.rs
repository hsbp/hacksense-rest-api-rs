extern crate serde;
extern crate askama;

use serde::Serialize;
use askama::Template;

#[derive(Queryable, Serialize, Template)]
#[template(path = "status.xml")]
pub struct Event {
    pub id: String,
    pub what: bool,
    pub when: String,
}
