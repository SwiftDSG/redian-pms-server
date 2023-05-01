use mongodb::{
    bson::{doc, from_document, oid::ObjectId, DateTime},
    Collection, Database,
};
pub struct Project {
    _id: Option<ObjectId>,
    customer_id: ObjectId,
    name: String,
    code: String,
    period: Option<ProjectPeriod>,
}
pub struct ProjectPeriod {
    start: DateTime,
    end: DateTime,
}
pub struct ProjectMember {
    _id: ObjectId,
    role_id: Vec<ObjectId>,
}
