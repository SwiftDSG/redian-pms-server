use mongodb::bson::{oid::ObjectId, DateTime};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct ProjectProgressReport {
    _id: Option<ObjectId>,
    project_id: ObjectId,
    date: DateTime,
    actual: Vec<ProjectProgressReportActual>,
    plan: Option<Vec<ProjectProgressReportPlan>>,
    documentation: Option<Vec<ProjectProgressReportDocumentation>>,
    weather: Option<Vec<ProjectProgressReportWeather>>,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct ProjectProgressReportActual {
    task_id: ObjectId,
    value: f64,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct ProjectProgressReportPlan {
    task_id: ObjectId,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct ProjectProgressReportDocumentation {
    image_url: String,
    description: Option<String>,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct ProjectProgressReportWeather {
    time: [u8; 2],
    condition: String,
}
