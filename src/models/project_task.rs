use mongodb::bson::oid::ObjectId;

pub struct ProjectTask {
    _id: Option<ObjectId>,
    name: String,
}
pub struct ProjectTaskVolume {
    value: usize,
    unit: String,
}
