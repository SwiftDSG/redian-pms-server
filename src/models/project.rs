use mongodb::{
    bson::{doc, from_document, oid::ObjectId, DateTime},
    Collection, Database,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct Project {
    _id: Option<ObjectId>,
    customer_id: ObjectId,
    name: String,
    code: String,
    period: Option<ProjectPeriod>,
    member: Vec<ProjectMember>,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct ProjectPeriod {
    start: DateTime,
    end: DateTime,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct ProjectMember {
    _id: ObjectId,
    role_id: Vec<ObjectId>,
}
