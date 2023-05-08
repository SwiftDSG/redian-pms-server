use crate::database::get_db;

use chrono::Utc;
use futures::stream::StreamExt;
use mongodb::{
    bson::{doc, from_document, oid::ObjectId, to_bson, DateTime},
    Collection, Database,
};
use serde::{Deserialize, Serialize};

use super::project::{Project, ProjectPeriod};

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum ProjectTaskStatusKind {
    Running,
    Paused,
    Pending,
    Finished,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ProjectTask {
    pub _id: Option<ObjectId>,
    pub project_id: ObjectId,
    pub area_id: ObjectId,
    pub name: String,
    pub period: Option<ProjectPeriod>,
    pub status: Vec<ProjectTaskStatus>,
    pub volume: ProjectTaskVolume,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct ProjectTaskPeriodRequest {
    pub start: DateTime,
    pub end: DateTime,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct ProjectTaskVolume {
    pub value: usize,
    pub unit: String,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct ProjectTaskStatus {
    pub kind: ProjectTaskStatusKind,
    pub time: DateTime,
    pub message: Option<String>,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct ProjectTaskStatusRequest {
    pub kind: ProjectTaskStatusKind,
    pub message: Option<String>,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct ProjectTaskResponse {
    pub _id: ObjectId,
    pub project: ProjectTaskProjectResponse,
    pub area: ProjectTaskAreaResponse,
    pub name: String,
    pub period: Option<ProjectPeriod>,
    pub status: Vec<ProjectTaskStatus>,
    pub volume: ProjectTaskVolume,
    pub progress: f64,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct ProjectTaskProjectResponse {
    pub _id: ObjectId,
    pub name: String,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct ProjectTaskAreaResponse {
    pub _id: ObjectId,
    pub name: String,
}
#[derive(Debug)]
pub struct ProjectTaskQuery {
    pub _id: Option<ObjectId>,
    pub limit: Option<usize>,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct ProjectTaskRequest {
    pub area_id: ObjectId,
    pub name: String,
    pub volume: ProjectTaskVolume,
}

impl ProjectTask {
    pub async fn save(&mut self) -> Result<ObjectId, String> {
        let db: Database = get_db();
        let collection: Collection<ProjectTask> = db.collection::<ProjectTask>("project-tasks");

        self._id = Some(ObjectId::new());

        if let Ok(Some(project)) = Project::find_by_id(&self.project_id).await {
            if project.area.is_some() && project.area.unwrap().iter().any(|a| a._id == self.area_id)
            {
                collection
                    .insert_one(self, None)
                    .await
                    .map_err(|_| "INSERTING_FAILED".to_string())
                    .map(|result| result.inserted_id.as_object_id().unwrap())
            } else {
                return Err("PROJECT_AREA_NOT_FOUND".to_string());
            }
        } else {
            return Err("PROJECT_NOT_FOUND".to_string());
        }
    }
    pub async fn update_period(&mut self, period: ProjectPeriod) -> Result<ObjectId, String> {
        let db: Database = get_db();
        let collection: Collection<ProjectTask> = db.collection::<ProjectTask>("project-tasks");

        if period.start.timestamp_millis() >= period.end.timestamp_millis() {
            return Err("INVALID_PERIOD".to_string());
        }

        self.period = Some(period);

        collection
            .update_one(
                doc! { "_id": self._id.unwrap() },
                doc! { "$set": to_bson::<ProjectTask>(self).unwrap()},
                None,
            )
            .await
            .map_err(|_| "UPDATE_FAILED".to_string())
            .map(|_| self._id.unwrap())
    }
    pub async fn update_status(
        &mut self,
        status: ProjectTaskStatusKind,
        message: Option<String>,
    ) -> Result<ObjectId, String> {
        let db: Database = get_db();
        let collection: Collection<ProjectTask> = db.collection::<ProjectTask>("project-tasks");

        self.status.push(ProjectTaskStatus {
            kind: status,
            time: DateTime::from_millis(Utc::now().timestamp_millis()),
            message,
        });

        collection
            .update_one(
                doc! { "_id": self._id.unwrap() },
                doc! { "$set": to_bson::<ProjectTask>(self).unwrap()},
                None,
            )
            .await
            .map_err(|_| "UPDATE_FAILED".to_string())
            .map(|_| self._id.unwrap())
    }
    pub async fn delete_by_id(_id: &ObjectId) -> Result<u64, String> {
        let db: Database = get_db();
        let collection: Collection<ProjectTask> = db.collection::<ProjectTask>("project-tasks");

        collection
            .delete_one(doc! { "_id": _id }, None)
            .await
            .map_err(|_| "PROJECT_TASK_NOT_FOUND".to_string())
            .map(|result| result.deleted_count)
    }
    pub async fn find_by_id(_id: &ObjectId) -> Result<Option<ProjectTask>, String> {
        let db: Database = get_db();
        let collection: Collection<ProjectTask> = db.collection::<ProjectTask>("project-tasks");

        collection
            .find_one(doc! { "_id": _id }, None)
            .await
            .map_err(|_| "ROLE_NOT_FOUND".to_string())
    }
    pub async fn find_detail_by_id(_id: &ObjectId) -> Result<Option<ProjectTaskResponse>, String> {
        let db: Database = get_db();
        let collection: Collection<ProjectTask> = db.collection::<ProjectTask>("project-tasks");

        let mut pipeline: Vec<mongodb::bson::Document> = vec![
            doc! {
                "$match": {
                    "$expr": {
                        "$eq": ["$_id", to_bson::<ObjectId>(_id).unwrap()]
                    }
                }
            },
            doc! {
                "$lookup": {
                    "from": "project-reports",
                    "as": "reports",
                    "let": {
                        "task_id": "$_id"
                    },
                    "pipeline": [
                        {
                            "$match": {
                                "$expr": {
                                    "$in": ["$$task_id", "$actual.task_id"]
                                }
                            },
                        },
                        {
                            "$group": {
                                "_id": to_bson::<Option<String>>(&Option::<String>::None).unwrap(),
                                "progress": {
                                    "$sum": {
                                        "$cond": [
                                            {
                                                "$gte": [
                                                    {
                                                        "$indexOfArray": ["$actual.task_id", "$$task_id"]
                                                    },
                                                    0
                                                ]
                                            },
                                            {
                                                "$arrayElemAt": [
                                                    "$actual.value",
                                                    {
                                                        "$indexOfArray": ["$actual.task_id", "$$task_id"]
                                                    }
                                                ]
                                            },
                                            0
                                        ]
                                    }
                                }
                            }
                        }
                    ]
                }
            },
            doc! {
                "$project": {
                    "project": "$project",
                    "area": "$area",
                    "name": "$name",
                    "period": "$period",
                    "status": "$status",
                    "volume": "$volume",
                    "progress": {
                        "$arrayElemAt": [
                            "$reports.progress",
                            0
                        ]
                    },
                }
            },
        ];

        if let Ok(mut cursor) = collection.aggregate(pipeline, None).await {
            if let Some(Ok(doc)) = cursor.next().await {
                let task: ProjectTaskResponse = from_document::<ProjectTaskResponse>(doc).unwrap();
                Ok(Some(task))
            } else {
                Ok(None)
            }
        } else {
            Err("USER_NOT_FOUND".to_string())
        }
    }
}
