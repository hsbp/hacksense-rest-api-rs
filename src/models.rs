#[derive(Queryable)]
pub struct Event {
    pub id: String,
    pub what: bool,
    pub when: String,
}
