use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Serialize};

use super::project::ProjectPeriod;

#[derive(Debug, Deserialize, Serialize)]
pub struct ProjectTask {
    _id: Option<ObjectId>,
    name: String,
    period: Option<ProjectPeriod>,
    volume: ProjectTaskVolume,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct ProjectTaskVolume {
    value: usize,
    unit: String,
}
