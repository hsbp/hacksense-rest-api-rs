extern crate serde;

use serde::Serialize;

#[derive(Queryable, Serialize)]
pub struct Event {
    pub id: String,
    pub what: bool,
    pub when: String,
}
