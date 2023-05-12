use crate::database::get_db;

use async_recursion::async_recursion;
use chrono::Utc;
use futures::stream::StreamExt;
use mongodb::{
    bson::{doc, from_document, oid::ObjectId, to_bson, DateTime, Document},
    Collection, Database,
};
use serde::{Deserialize, Serialize};

use super::project::{Project, ProjectStatusKind};

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProjectTaskStatusKind {
    Running,
    Paused,
    Pending,
    Finished,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ProjectTask {
    pub _id: Option<ObjectId>,
    pub project_id: ObjectId,
    pub area_id: ObjectId,
    pub task_id: Option<ObjectId>,
    pub user_id: Option<Vec<ObjectId>>,
    pub name: String,
    pub description: Option<String>,
    pub period: Option<ProjectTaskPeriod>,
    pub status: Vec<ProjectTaskStatus>,
    pub volume: Option<ProjectTaskVolume>,
    pub value: f64,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ProjectTaskPeriod {
    pub start: DateTime,
    pub end: DateTime,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ProjectTaskVolume {
    pub value: usize,
    pub unit: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
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
    pub sub_task: Option<Vec<ProjectTaskSubTaskResponse>>,
    pub name: String,
    pub period: Option<ProjectTaskPeriod>,
    pub status: Vec<ProjectTaskStatus>,
    pub volume: Option<ProjectTaskVolume>,
    pub value: f64,
    pub progress: f64,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct ProjectTaskSubTaskResponse {
    pub _id: ObjectId,
    pub name: String,
    pub period: Option<ProjectTaskPeriod>,
    pub status: Vec<ProjectTaskStatus>,
    pub volume: Option<ProjectTaskVolume>,
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
    pub project_id: Option<ObjectId>,
    pub task_id: Option<ObjectId>,
    pub area_id: Option<ObjectId>,
    pub limit: Option<usize>,
    pub base: bool,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct ProjectTaskRequest {
    pub area_id: Option<ObjectId>,
    pub user_id: Option<Vec<ObjectId>>,
    pub name: String,
    pub description: Option<String>,
    pub volume: Option<ProjectTaskVolume>,
    pub value: f64,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct ProjectTaskPeriodRequest {
    pub start: DateTime,
    pub end: DateTime,
}

impl ProjectTask {
    pub async fn save(&mut self) -> Result<ObjectId, String> {
        let db: Database = get_db();
        let collection: Collection<ProjectTask> = db.collection::<ProjectTask>("project-tasks");

        self._id = Some(ObjectId::new());

        if let Some(task_id) = self.task_id {
            let mut parent_task = Self::find_by_id(&task_id)
                .await
                .map_err(|_| "PROJECT_TASK_NOT_FOUND".to_string())?
                .ok_or_else(|| "PROJECT_TASK_NOT_FOUND".to_string())?;

            if parent_task.volume.is_some() {
                parent_task.volume = None;
            }
            if parent_task.user_id.is_some() {
                parent_task.user_id = None;
            }

            parent_task
                .update()
                .await
                .map_err(|_| "PROJECT_TASK_UPDATE_FAILED".to_string())?;
            self.area_id = parent_task.area_id;
        }

        if let Ok(Some(project)) = Project::find_by_id(&self.project_id).await {
            if project.area.is_some() && project.area.unwrap().iter().any(|a| a._id == self.area_id)
            {
                collection
                    .insert_one(self, None)
                    .await
                    .map_err(|_| "INSERTING_FAILED".to_string())
                    .map(|result| result.inserted_id.as_object_id().unwrap())
            } else {
                Err("PROJECT_AREA_NOT_FOUND".to_string())
            }
        } else {
            Err("PROJECT_NOT_FOUND".to_string())
        }
    }
    pub async fn update(&self) -> Result<ObjectId, String> {
        let db: Database = get_db();
        let collection: Collection<ProjectTask> = db.collection::<ProjectTask>("project-tasks");

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
    pub async fn update_period(&mut self, period: ProjectTaskPeriod) -> Result<ObjectId, String> {
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
    #[async_recursion]
    pub async fn update_status(
        &mut self,
        status: ProjectTaskStatusKind,
        message: Option<String>,
    ) -> Result<ObjectId, String> {
        let db = get_db();
        let collection = db.collection::<ProjectTask>("project-tasks");

        self.status.insert(
            0,
            ProjectTaskStatus {
                kind: status.clone(),
                time: DateTime::from_millis(Utc::now().timestamp_millis()),
                message,
            },
        );

        let mut finished_parent_task = None;
        if status == ProjectTaskStatusKind::Finished {
            if self.task_id.is_some() {
                let tasks = Self::find_many(&ProjectTaskQuery {
                    _id: None,
                    project_id: None,
                    task_id: self.task_id,
                    area_id: None,
                    limit: None,
                    base: false,
                })
                .await?
                .ok_or_else(|| "UPDATE_FAILED".to_string())?;

                if tasks.iter().all(|task| {
                    task._id == self._id
                        || task.status.get(0).unwrap().kind == ProjectTaskStatusKind::Finished
                }) {
                    finished_parent_task = self.task_id;
                } else {
                    finished_parent_task = None
                }
            } else {
                println!("{:#?}", self);
                let tasks = Self::find_many(&ProjectTaskQuery {
                    _id: None,
                    project_id: None,
                    task_id: None,
                    area_id: None,
                    limit: None,
                    base: true,
                })
                .await?
                .ok_or_else(|| "UPDATE_FAILED".to_string())?;

                if tasks.iter().all(|task| {
                    task._id == self._id
                        || task.status.get(0).unwrap().kind == ProjectTaskStatusKind::Finished
                }) {
                    let mut project = Project::find_by_id(&self.project_id)
                        .await?
                        .ok_or_else(|| "UPDATE_FAILED".to_string())?;

                    project
                        .update_status(ProjectStatusKind::Finished, None)
                        .await?;
                }
            }
        }

        collection
            .update_one(
                doc! { "_id": self._id.unwrap() },
                doc! { "$set": to_bson(self).unwrap()},
                None,
            )
            .await
            .map_err(|_| "UPDATE_FAILED".to_string())?;

        if let Some(finished_parent_task) = finished_parent_task {
            Self::find_by_id(&finished_parent_task)
                .await?
                .ok_or_else(|| "PROJECT_TASK_NOT_FOUND".to_string())?
                .update_status(ProjectTaskStatusKind::Finished, None)
                .await
        } else {
            Ok(self._id.unwrap())
        }
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
    pub async fn find_many(query: &ProjectTaskQuery) -> Result<Option<Vec<ProjectTask>>, String> {
        let db: Database = get_db();
        let collection: Collection<ProjectTask> = db.collection::<ProjectTask>("project-tasks");

        let mut tasks: Vec<ProjectTask> = Vec::<ProjectTask>::new();
        let mut pipeline: Vec<Document> = Vec::<Document>::new();
        let mut queries: Vec<Document> = Vec::<Document>::new();

        if let Some(_id) = query._id {
            queries.push(doc! {
                "$eq": [ "$_id", to_bson::<ObjectId>(&_id).unwrap() ]
            });
        }
        if let Some(_id) = query.project_id {
            queries.push(doc! {
                "$eq": [ "$project_id", to_bson::<ObjectId>(&_id).unwrap() ]
            });
        }
        if let Some(_id) = query.area_id {
            queries.push(doc! {
                "$eq": [ "$area_id", to_bson::<ObjectId>(&_id).unwrap() ]
            });
        }
        if let Some(_id) = query.task_id {
            queries.push(doc! {
                "$eq": [ "$task_id", to_bson::<ObjectId>(&_id).unwrap() ]
            });
        } else if query.base {
            queries.push(doc! {
                "$eq": [ "$task_id", to_bson::<Option<ObjectId>>(&None).unwrap() ]
            });
        }

        pipeline.push(doc! {
            "$match": {
                "$expr": {
                    "$and": queries
                }
            }
        });

        if let Some(limit) = query.limit {
            pipeline.push(doc! {
                "$limit": to_bson::<usize>(&limit).unwrap()
            });
        }

        if let Ok(mut cursor) = collection.aggregate(pipeline, None).await {
            while let Some(Ok(doc)) = cursor.next().await {
                let task: ProjectTask = from_document::<ProjectTask>(doc).unwrap();
                tasks.push(task);
            }
            if !tasks.is_empty() {
                Ok(Some(tasks))
            } else {
                Ok(None)
            }
        } else {
            Err("PROJECT_TASK_NOT_FOUND".to_string())
        }
    }
    pub async fn find_by_id(_id: &ObjectId) -> Result<Option<ProjectTask>, String> {
        let db: Database = get_db();
        let collection: Collection<ProjectTask> = db.collection::<ProjectTask>("project-tasks");

        collection
            .find_one(doc! { "_id": _id }, None)
            .await
            .map_err(|_| "PROJECT_TASK_NOT_FOUND".to_string())
    }
    pub async fn find_detail_by_id(_id: &ObjectId) -> Result<Option<ProjectTaskResponse>, String> {
        let db: Database = get_db();
        let collection: Collection<ProjectTask> = db.collection::<ProjectTask>("project-tasks");

        let pipeline: Vec<Document> = vec![
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
                    "as": "report",
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
                "$lookup": {
                    "from": "project-tasks",
                    "as": "sub_task",
                    "let": {
                        "task_id": "$_id"
                    },
                    "pipeline": [
                        {
                            "$match": {
                                "$expr": {
                                    "$eq": ["$task_id", "$$task_id"]
                                }
                            },
                        },
                        {
                            "$lookup": {
                                "from": "project-reports",
                                "as": "report",
                                "let": {
                                    "sub_task_id": "$_id"
                                },
                                "pipeline": [
                                    {
                                        "$match": {
                                            "$expr": {
                                                "$in": ["$$sub_task_id", "$actual.task_id"]
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
                                                                    "$indexOfArray": ["$actual.task_id", "$$sub_task_id"]
                                                                },
                                                                0
                                                            ]
                                                        },
                                                        {
                                                            "$arrayElemAt": [
                                                                "$actual.value",
                                                                {
                                                                    "$indexOfArray": ["$actual.task_id", "$$sub_task_id"]
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
                        {
                            "$project": {
                                "name": "$name",
                                "period": "$period",
                                "status": "$status",
                                "volume": "$volume",
                                "progress": {
                                    "$cond": [
                                        {
                                            "$gt": [
                                                {
                                                    "$size": "$report"
                                                },
                                                0
                                            ]
                                        },
                                        {
                                            "$arrayElemAt": [
                                                "$report.progress",
                                                0
                                            ]
                                        },
                                        0
                                    ]
                                },
                            }
                        }
                    ]
                }
            },
            doc! {
                "$lookup": {
                    "from": "projects",
                    "as": "project",
                    "let": {
                        "project_id": "$project_id",
                        "area_id": "$area_id",
                    },
                    "pipeline": [
                        {
                            "$match": {
                                "$expr": {
                                    "$eq": ["$$project_id", "$_id"]
                                }
                            },
                        },
                        {
                            "$project": {
                                "name": "$name",
                                "area": {
                                    "$arrayElemAt": [
                                        {
                                            "$filter": {
                                                "input": "$area",
                                                "cond": {
                                                    "$eq": ["$$this._id", "$$area_id"]
                                                }
                                            }
                                        },
                                        0
                                    ]
                                }
                            }
                        }
                    ]
                }
            },
            doc! {
                "$project": {
                    "project": {
                        "$arrayElemAt": ["$project", 0]
                    },
                    "area": {
                        "_id": {
                            "$arrayElemAt": ["$project.area._id", 0]
                        },
                        "name": {
                            "$arrayElemAt": ["$project.area.name", 0]
                        },
                    },
                    "sub_task": {
                        "$cond": [
                            {
                                "$gt": [
                                    {
                                        "$size": "$sub_task"
                                    },
                                    0
                                ]
                            },
                            "$sub_task",
                            to_bson::<Option<ObjectId>>(&None).unwrap()
                        ]
                    },
                    "name": "$name",
                    "period": "$period",
                    "status": "$status",
                    "volume": "$volume",
                    "value": "$value",
                    "progress": {
                        "$cond": [
                            {
                                "$gt": [
                                    {
                                        "$size": "$report"
                                    },
                                    0
                                ]
                            },
                            {
                                "$arrayElemAt": [
                                    "$report.progress",
                                    0
                                ]
                            },
                            0
                        ]
                    },
                }
            },
        ];

        if let Ok(mut cursor) = collection.aggregate(pipeline, None).await {
            if let Some(Ok(doc)) = cursor.next().await {
                let task = from_document::<ProjectTaskResponse>(doc).unwrap();
                Ok(Some(task))
            } else {
                Ok(None)
            }
        } else {
            Err("PROJECT_TASK_NOT_FOUND".to_string())
        }
    }
}
