use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct ProjectRole {
    _id: ObjectId,
    project_id: ObjectId,
    name: String,
    permission: Vec<String>,
}
