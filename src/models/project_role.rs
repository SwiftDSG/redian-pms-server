use mongodb::bson::oid::ObjectId;

pub struct ProjectRole {
    _id: ObjectId,
    project_id: ObjectId,
    name: String,
    permission: Vec<String>,
}
