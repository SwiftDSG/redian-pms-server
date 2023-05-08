use crate::database::get_db;

use mongodb::{
    bson::{doc, oid::ObjectId, DateTime},
    Collection, Database,
};
use serde::{Deserialize, Serialize};

use super::{
    project::Project,
    project_task::{ProjectTask, ProjectTaskStatus, ProjectTaskStatusKind},
};

#[derive(Debug, Deserialize, Serialize)]
pub struct ProjectProgressReport {
    pub _id: Option<ObjectId>,
    pub project_id: ObjectId,
    pub date: DateTime,
    pub time: [[u8; 2]; 2],
    pub actual: Option<Vec<ProjectProgressReportActual>>,
    pub plan: Option<Vec<ProjectProgressReportPlan>>,
    pub documentation: Option<Vec<ProjectProgressReportDocumentation>>,
    pub weather: Option<Vec<ProjectProgressReportWeather>>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ProjectProgressReportActual {
    pub task_id: ObjectId,
    pub value: f64,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct ProjectProgressReportPlan {
    pub task_id: ObjectId,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct ProjectProgressReportDocumentation {
    pub image_url: String,
    pub description: Option<String>,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct ProjectProgressReportWeather {
    pub time: [u8; 2],
    pub condition: String,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct ProjectProgressReportRequest {
    pub date: DateTime,
    pub time: [[u8; 2]; 2],
    pub actual: Option<Vec<ProjectProgressReportActual>>,
    pub plan: Option<Vec<ProjectProgressReportPlan>>,
    pub weather: Option<Vec<ProjectProgressReportWeather>>,
}

impl ProjectProgressReport {
    pub async fn save(&mut self) -> Result<ObjectId, String> {
        let db: Database = get_db();
        let collection: Collection<ProjectProgressReport> =
            db.collection::<ProjectProgressReport>("project-reports");

        self._id = Some(ObjectId::new());

        if let Ok(Some(_)) = Project::find_by_id(&self.project_id).await {
            if let Some(actual) = &self.actual {
                let mut actual: Vec<ProjectProgressReportActual> = actual.clone();
                for i in actual.iter_mut() {
                    if let Ok(Some(task)) = ProjectTask::find_detail_by_id(&i.task_id).await {
                        let remain = 100.0 - task.progress;
                        if remain <= i.value {
                            i.value = remain;
                            if let Ok(Some(mut task)) = ProjectTask::find_by_id(&i.task_id).await {
                                if task
                                    .update_status(ProjectTaskStatusKind::Finished, None)
                                    .await
                                    .is_err()
                                {
                                    return Err("TASK_UPDATE_FAILED".to_string());
                                }
                            }
                        } else {
                            let status: &ProjectTaskStatus = task.status.get(0).unwrap();
                            if let ProjectTaskStatusKind::Pending = status.kind {
                                if let Ok(Some(mut task)) =
                                    ProjectTask::find_by_id(&i.task_id).await
                                {
                                    if task
                                        .update_status(ProjectTaskStatusKind::Running, None)
                                        .await
                                        .is_err()
                                    {
                                        return Err("TASK_UPDATE_FAILED".to_string());
                                    }
                                }
                            } else if let ProjectTaskStatusKind::Paused = status.kind {
                                if let Ok(Some(mut task)) =
                                    ProjectTask::find_by_id(&i.task_id).await
                                {
                                    if task
                                        .update_status(ProjectTaskStatusKind::Running, None)
                                        .await
                                        .is_err()
                                    {
                                        return Err("TASK_UPDATE_FAILED".to_string());
                                    }
                                }
                            }
                        }
                    }
                }

                self.actual = Some(actual);
            }

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
