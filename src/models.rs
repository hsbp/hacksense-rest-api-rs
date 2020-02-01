extern crate serde;
extern crate askama;

use serde::Serialize;
use askama::Template;

use crate::schema::events;

#[derive(Queryable, Serialize, Template)]
#[template(path = "status.xml")]
pub struct Event {
    pub id: String,
    pub what: bool,
    pub when: String,
}

#[derive(Insertable)]
#[table_name="events"]
pub struct Submission<'a> {
    pub id: &'a str,
    pub what: bool,
    pub when: String,
}

#[derive(Template)]
#[template(path = "status.html")]
pub struct Status<'a> {
	pub open_closed: &'a str,
	pub when: &'a str,
}

impl Event {
    pub fn get_status(&self) -> Status {
        Status { open_closed: if self.what { "open" } else { "closed" }, when: &self.when }
    }
}
