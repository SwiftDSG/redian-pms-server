use crate::database::get_db;

use mongodb::{
    bson::{doc, oid::ObjectId, DateTime},
    Collection, Database,
};
use serde::{Deserialize, Serialize};

use super::project::{Project, ProjectStatusKind};

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum ProjectIncidentReportKind {
    FirstAid,
    LostTimeInjury,
    Fatal,
    PropertyDamage,
    Environmental,
    NearMiss,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ProjectIncidentReport {
    pub _id: Option<ObjectId>,
    pub project_id: ObjectId,
    pub user_id: Vec<ObjectId>,
    pub date: DateTime,
    pub kind: ProjectIncidentReportKind,
}
#[derive(Debug, Deserialize)]
pub struct ProjectIncidentReportRequest {
    pub project_id: ObjectId,
    pub user_id: Vec<ObjectId>,
    pub kind: ProjectIncidentReportKind,
}
#[derive(Deserialize)]
pub struct ProjectIncidentReportRequestQuery {
    pub breakdown: bool,
}

impl ProjectIncidentReport {
    pub async fn save(&mut self, breakdown: bool) -> Result<ObjectId, String> {
        let db: Database = get_db();
        let collection: Collection<ProjectIncidentReport> =
            db.collection::<ProjectIncidentReport>("project-incidents");

        self._id = Some(ObjectId::new());

        if let Ok(Some(mut project)) = Project::find_by_id(&self.project_id).await {
            let result = collection
                .insert_one(self, None)
                .await
                .map_err(|_| "INSERTING_FAILED".to_string())
                .map(|result| result.inserted_id.as_object_id().unwrap())?;

            if breakdown {
                project
                    .update_status(ProjectStatusKind::Breakdown, None)
                    .await
                    .map_err(|_| "PROJECT_STATUS_UPDATE_FAILED".to_string())?;
            }
            Ok(result)
        } else {
            Err("PROJECT_NOT_FOUND".to_string())
        }
    }
}
