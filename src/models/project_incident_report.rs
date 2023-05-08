use crate::database::get_db;

use mongodb::{
    bson::{doc, oid::ObjectId, DateTime},
    Collection, Database,
};
use serde::{Deserialize, Serialize};

use super::project::Project;

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
    _id: Option<ObjectId>,
    project_id: ObjectId,
    date: DateTime,
    kind: ProjectIncidentReportKind,
    involved: Vec<ObjectId>,
}

impl ProjectIncidentReport {
    pub async fn save(&mut self) -> Result<ObjectId, String> {
        let db: Database = get_db();
        let collection: Collection<ProjectIncidentReport> =
            db.collection::<ProjectIncidentReport>("project-incidents");

        self._id = Some(ObjectId::new());

        if let Ok(Some(_)) = Project::find_by_id(&self.project_id).await {
            collection
                .insert_one(self, None)
                .await
                .map_err(|_| "INSERTING_FAILED".to_string())
                .map(|result| result.inserted_id.as_object_id().unwrap())
        } else {
            return Err("PROJECT_NOT_FOUND".to_string());
        }
    }
}
