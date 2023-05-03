use mongodb::bson::{oid::ObjectId, DateTime};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum ProjectSafetyReportIncidentKind {
    FirstAid,
    LostTimeInjury,
    Fatal,
    PropertyDamage,
    Environmental,
    NearMiss,
}
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum ProjectSafetyReportStatus {
    OnGoing,
    Cleared,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ProjectSafetyReport {
    _id: Option<ObjectId>,
    project_id: ObjectId,
    date: DateTime,
    status: ProjectSafetyReportStatus,
    incident: Vec<ProjectSafetyReportIncident>,
    period: Option<ProjectSafetyReportPeriod>,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct ProjectSafetyReportIncident {
    kind: ProjectSafetyReportIncidentKind,
    involved: Vec<ObjectId>,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct ProjectSafetyReportPeriod {
    start: DateTime,
    end: Option<DateTime>,
}
