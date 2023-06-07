use crate::database::get_db;

use actix_multipart::form::{tempfile::TempFile, MultipartForm};
use mongodb::{
    bson::{doc, oid::ObjectId, to_bson, DateTime},
    Collection, Database,
};
use serde::{Deserialize, Serialize};

use super::{
    project::{Project, ProjectStatusKind},
    project_task::{ProjectTask, ProjectTaskStatusKind},
};

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum ProjectProgressReportWeatherKind {
    Sunny,
    Cloudy,
    Rainy,
    Snowy,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ProjectProgressReport {
    pub _id: Option<ObjectId>,
    pub project_id: ObjectId,
    pub date: DateTime,
    pub time: [[usize; 2]; 2],
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
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ProjectProgressReportDocumentation {
    pub _id: Option<ObjectId>,
    pub description: Option<String>,
    pub extension: Option<String>,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct ProjectProgressReportWeather {
    pub time: [usize; 2],
    pub kind: ProjectProgressReportWeatherKind,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct ProjectProgressReportRequest {
    pub time: [[usize; 2]; 2],
    pub actual: Option<Vec<ProjectProgressReportActual>>,
    pub plan: Option<Vec<ProjectProgressReportPlan>>,
    pub weather: Option<Vec<ProjectProgressReportWeather>>,
    pub documentation: Option<Vec<ProjectProgressReportDocumentation>>,
}
#[derive(Debug, MultipartForm)]
pub struct ProjectProgressReportDocumentationRequest {
    #[multipart(rename = "file")]
    pub files: Vec<TempFile>,
}

impl ProjectProgressReport {
    pub async fn save(&mut self) -> Result<ObjectId, String> {
        let db: Database = get_db();
        let collection = db.collection::<ProjectProgressReport>("project-reports");
        self._id = Some(ObjectId::new());

        let mut project = Project::find_by_id(&self.project_id)
            .await
            .map_err(|_| "PROJECT_NOT_FOUND".to_string())?
            .ok_or_else(|| "PROJECT_NOT_FOUND".to_string())?;

        let (start_time, end_time) = (self.time[0], self.time[1]);
        if start_time[0] > 23
            || start_time[1] > 59
            || end_time[0] > 23
            || end_time[1] > 59
            || (start_time[0] * 60 + start_time[1]) >= (end_time[0] * 60 + end_time[1])
        {
            return Err("PROJECT_REPORT_TIME_INVALID".to_string());
        }

        if let Some(documentation) = self.documentation.as_mut() {
            for i in documentation {
                i._id = Some(ObjectId::new());
            }
        }

        if let Some(actual) = self.actual.as_mut() {
            let mut invalid_task_index = Vec::<usize>::new();
            for (i, actual_task) in actual.iter_mut().enumerate() {
                if let Ok(Some(task)) = ProjectTask::find_detail_by_id(&actual_task.task_id).await {
                    if task.task.is_some() {
                        invalid_task_index.push(i);
                        continue;
                    }
                    let remain = 100.0 - task.progress;
                    if remain <= actual_task.value {
                        actual_task.value = remain;
                        let mut task = ProjectTask::find_by_id(&actual_task.task_id)
                            .await
                            .map_err(|_| "PROJECT_TASK_NOT_FOUND".to_string())?
                            .ok_or_else(|| "PROJECT_TASK_NOT_FOUND".to_string())?;
                        task.update_status(ProjectTaskStatusKind::Finished, None)
                            .await
                            .map_err(|_| "PROJECT_TASK_UPDATE_FAILED".to_string())?;
                    } else {
                        let status = task
                            .status
                            .first()
                            .ok_or_else(|| "PROJECT_TASK_STATUS_NOT_FOUND".to_string())?;
                        match status.kind {
                            ProjectTaskStatusKind::Pending => {
                                if project.status.get(0).unwrap().kind == ProjectStatusKind::Pending
                                {
                                    project
                                        .update_status(ProjectStatusKind::Running, None)
                                        .await
                                        .map_err(|_| "PROJECT_UPDATE_FAILED".to_string())?;
                                }
                                let mut task = ProjectTask::find_by_id(&actual_task.task_id)
                                    .await
                                    .map_err(|_| "PROJECT_TASK_NOT_FOUND".to_string())?
                                    .ok_or_else(|| "PROJECT_TASK_NOT_FOUND".to_string())?;
                                task.update_status(ProjectTaskStatusKind::Running, None)
                                    .await
                                    .map_err(|_| "PROJECT_TASK_UPDATE_FAILED".to_string())?;
                            }
                            ProjectTaskStatusKind::Paused => {
                                let mut task = ProjectTask::find_by_id(&actual_task.task_id)
                                    .await
                                    .map_err(|_| "PROJECT_TASK_NOT_FOUND".to_string())?
                                    .ok_or_else(|| "PROJECT_TASK_NOT_FOUND".to_string())?;
                                task.update_status(ProjectTaskStatusKind::Running, None)
                                    .await
                                    .map_err(|_| "PROJECT_TASK_UPDATE_FAILED".to_string())?;
                            }
                            _ => {}
                        }
                    }
                } else {
                    invalid_task_index.push(i);
                }
            }
            for i in invalid_task_index.iter() {
                actual.remove(*i);
            }
        }

        collection
            .insert_one(self, None)
            .await
            .map_err(|_| "INSERTING_FAILED".to_string())
            .map(|result| result.inserted_id.as_object_id().unwrap())
    }
    pub async fn update(&self) -> Result<ObjectId, String> {
        let db: Database = get_db();
        let collection: Collection<ProjectProgressReport> =
            db.collection::<ProjectProgressReport>("project-reports");

        collection
            .update_one(
                doc! { "_id": self._id.unwrap() },
                doc! { "$set": to_bson::<ProjectProgressReport>(self).unwrap()},
                None,
            )
            .await
            .map_err(|_| "UPDATE_FAILED".to_string())
            .map(|_| self._id.unwrap())
    }
    pub async fn find_by_id(_id: &ObjectId) -> Result<Option<ProjectProgressReport>, String> {
        let db: Database = get_db();
        let collection: Collection<ProjectProgressReport> =
            db.collection::<ProjectProgressReport>("project-reports");

        collection
            .find_one(doc! { "_id": _id }, None)
            .await
            .map_err(|_| "PROJECT_REPORT_NOT_FOUND".to_string())
    }
    pub async fn delete_by_id(_id: &ObjectId) -> Result<u64, String> {
        let db: Database = get_db();
        let collection: Collection<ProjectProgressReport> =
            db.collection::<ProjectProgressReport>("project-reports");

        collection
            .delete_one(doc! { "_id": _id }, None)
            .await
            .map_err(|_| "PROJECT_REPORT_NOT_FOUND".to_string())
            .map(|result| result.deleted_count)
    }
}
