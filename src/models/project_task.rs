use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Serialize};

use super::project::{ProjectPeriodActual, ProjectPeriodPlan};

#[derive(Debug, Deserialize, Serialize)]
pub struct ProjectTask {
    _id: Option<ObjectId>,
    name: String,
    period_plan: Option<ProjectPeriodPlan>,
    period_actual: Option<ProjectPeriodActual>,
    volume: ProjectTaskVolume,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct ProjectTaskVolume {
    value: usize,
    unit: String,
}
