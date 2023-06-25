use crate::database::get_db;

use async_recursion::async_recursion;
use chrono::Utc;
use futures::stream::StreamExt;
use mongodb::{
    bson::{doc, from_document, oid::ObjectId, to_bson, DateTime, Document},
    Collection, Database,
};
use serde::{Deserialize, Serialize};

use super::{
    project::{Project, ProjectAreaResponse, ProjectStatusKind},
    user::UserImage,
};

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProjectTaskStatusKind {
    Running,
    Paused,
    Pending,
    Finished,
}
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProjectTaskQueryKind {
    Root,       // Main tasks (does not have parent task)
    Dependency, // Tasks that have sub-tasks
    Base,       // Tasks that does not have sub-task
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
    pub _id: String,
    pub project: ProjectTaskProjectResponse,
    pub area: ProjectTaskAreaResponse,
    pub user: Option<Vec<ProjectTaskUserResponse>>,
    pub task: Option<Vec<ProjectTaskMinResponse>>,
    pub name: String,
    pub description: Option<String>,
    pub period: Option<ProjectTaskPeriodResponse>,
    pub status: Vec<ProjectTaskStatus>,
    pub volume: Option<ProjectTaskVolume>,
    pub value: f64,
    pub progress: f64,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct ProjectTaskMinResponse {
    pub _id: String,
    pub task_id: Option<ObjectId>,
    pub user: Option<Vec<ProjectTaskUserResponse>>,
    pub task: Option<Vec<ProjectTaskTaskResponse>>,
    pub name: String,
    pub period: Option<ProjectTaskPeriodResponse>,
    pub actual: Option<ProjectTaskPeriodResponse>,
    pub status: Vec<ProjectTaskStatus>,
    pub volume: Option<ProjectTaskVolume>,
    pub value: f64,
    pub progress: f64,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct ProjectTaskTaskResponse {
    pub _id: String,
    pub name: String,
    pub period: Option<ProjectTaskPeriodResponse>,
    pub status: Vec<ProjectTaskStatus>,
    pub volume: Option<ProjectTaskVolume>,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct ProjectTaskUserResponse {
    pub _id: String,
    pub name: String,
    pub image: Option<UserImage>,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct ProjectTaskProjectResponse {
    pub _id: String,
    pub name: String,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct ProjectTaskAreaResponse {
    pub _id: String,
    pub name: String,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct ProjectTaskPeriodResponse {
    pub start: String,
    pub end: String,
}
#[derive(Debug)]
pub struct ProjectTaskQuery {
    pub _id: Option<ObjectId>,
    pub project_id: Option<ObjectId>,
    pub task_id: Option<ObjectId>,
    pub area_id: Option<ObjectId>,
    pub limit: Option<usize>,
    pub kind: Option<ProjectTaskQueryKind>,
}
pub struct ProjectTaskTimelineQuery {
    pub project_id: ObjectId,
    pub area_id: Option<ObjectId>,
    pub task_id: Option<ObjectId>,
    pub status: Option<ProjectTaskStatusKind>,
    pub relative: bool,
    pub subtask: bool,
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
#[derive(Debug, Deserialize)]
pub struct ProjectTaskPeriodRequest {
    pub start: i64,
    pub end: i64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ProjectTaskDependency {
    pub _id: ObjectId,
    pub task_id: Option<ObjectId>,
    pub value: f64,
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

        let tasks = Self::find_many(&ProjectTaskQuery {
            _id: None,
            project_id: Some(self.project_id),
            task_id: None,
            area_id: None,
            limit: None,
            kind: None,
        })
        .await
        .map_err(|_| "PROJECT_TASK_NOT_FOUND".to_string())?
        .ok_or("PROJECT_TASK_NOT_FOUND")?;
        let mut task_id: Vec<ObjectId> = Vec::new();

        for task in tasks.iter() {
            if let Some(_id) = task.task_id {
                if !task_id.contains(&_id) {
                    task_id.push(_id);
                }
            }
        }

        if task_id.contains(&self._id.unwrap()) {
            return Err("PROJECT_TASK_DEPENDENCY".to_string());
        }

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
                    kind: None,
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
                let tasks = Self::find_many(&ProjectTaskQuery {
                    _id: None,
                    project_id: None,
                    task_id: None,
                    area_id: None,
                    limit: None,
                    kind: Some(ProjectTaskQueryKind::Root),
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
    pub async fn delete_many_by_area_id(_id: &ObjectId) -> Result<u64, String> {
        let db: Database = get_db();
        let collection: Collection<ProjectTask> = db.collection::<ProjectTask>("project-tasks");

        collection
            .delete_many(doc! { "area_id": _id }, None)
            .await
            .map_err(|_| "PROJECT_TASK_NOT_FOUND".to_string())
            .map(|result| result.deleted_count)
    }
    pub async fn delete_many_by_task_id(_id: &ObjectId) -> Result<u64, String> {
        let db: Database = get_db();
        let collection: Collection<ProjectTask> = db.collection::<ProjectTask>("project-tasks");

        collection
            .delete_many(doc! { "task_id": _id }, None)
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

        if let Some(_id) = &query._id {
            queries.push(doc! {
                "$eq": [ "$_id", to_bson::<ObjectId>(_id).unwrap() ]
            });
        }
        if let Some(_id) = &query.project_id {
            queries.push(doc! {
                "$eq": [ "$project_id", to_bson::<ObjectId>(_id).unwrap() ]
            });
        }
        if let Some(_id) = &query.area_id {
            queries.push(doc! {
                "$eq": [ "$area_id", to_bson::<ObjectId>(_id).unwrap() ]
            });
        }
        if let Some(_id) = &query.task_id {
            queries.push(doc! {
                "$eq": [ "$task_id", to_bson::<ObjectId>(_id).unwrap() ]
            });
        }
        if let Some(kind) = query.kind.clone() {
            if kind == ProjectTaskQueryKind::Root {
                queries.push(doc! {
                    "$eq": [ "$task_id", to_bson::<Option<ObjectId>>(&None).unwrap() ]
                });
            } else {
                let mut task_id: Vec<ObjectId> = Vec::new();
                if let Ok(mut cursor) = collection
                    .find(
                        doc! {
                            "project_id": to_bson::<Option<ObjectId>>(&query.project_id).unwrap()
                        },
                        None,
                    )
                    .await
                {
                    while let Some(Ok(task)) = cursor.next().await {
                        if let Some(_id) = task.task_id {
                            if !task_id.contains(&_id) {
                                task_id.push(_id);
                            }
                        }
                    }
                }

                if kind == ProjectTaskQueryKind::Dependency {
                    queries.push(doc! {
                        "$in": ["$_id", to_bson::<Vec<ObjectId>>(&task_id).unwrap()]
                    });
                } else {
                    queries.push(doc! {
                        "$ne": [
                            {
                                "$in": ["$_id", to_bson::<Vec<ObjectId>>(&task_id).unwrap()]
                            },
                            to_bson::<bool>(&true).unwrap()
                        ]
                    });
                }
            }
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
    pub async fn find_many_timeline(
        query: &ProjectTaskTimelineQuery,
    ) -> Result<Option<Vec<ProjectTaskMinResponse>>, String> {
        let db: Database = get_db();
        let collection: Collection<ProjectTask> = db.collection::<ProjectTask>("project-tasks");

        let mut dependencies: Vec<ProjectTask> = Vec::new();
        let mut task_id: Vec<ObjectId> = Vec::new();

        if !query.relative {
            if let Ok(Some(tasks)) = Self::find_many(&ProjectTaskQuery {
                _id: None,
                project_id: Some(query.project_id),
                task_id: None,
                area_id: None,
                limit: None,
                kind: Some(ProjectTaskQueryKind::Dependency),
            })
            .await
            {
                dependencies = tasks;
                for task in dependencies.iter() {
                    if !task_id.contains(&task._id.unwrap()) {
                        task_id.push(task._id.unwrap());
                    }
                }
            }
        }

        let mut pipeline: Vec<Document> = Vec::<Document>::new();
        let mut queries: Vec<Document> = Vec::<Document>::new();

        queries.push(doc! {
            "$eq": [ "$project_id", to_bson::<ObjectId>(&query.project_id).unwrap() ]
        });
        queries.push(doc! {
            "$ne": [
                {
                    "$in": ["$_id", to_bson::<Vec<ObjectId>>(&task_id).unwrap()]
                },
                to_bson::<bool>(&true).unwrap()
            ]
        });
        if let Some(_id) = query.area_id {
            queries.push(doc! {
                "$eq": [ "$area_id", to_bson::<ObjectId>(&_id).unwrap() ]
            });
        }
        if let Some(_id) = query.task_id {
            queries.push(doc! {
                "$eq": [ "$task_id", to_bson::<ObjectId>(&_id).unwrap() ]
            });
        }
        if let Some(status) = query.status.clone() {
            queries.push(doc! {
                "$ne": [
                    {
                        "$arrayElemAt": ["$status.kind", 0]
                    },
                    to_bson::<ProjectTaskStatusKind>(&status).unwrap()
                ]
            });
        }

        pipeline.push(doc! {
            "$match": {
                "$expr": {
                    "$and": queries
                }
            }
        });
        pipeline.push(doc! {
            "$lookup": {
                "from": "project-reports",
                "let": {
                    "task_id": "$_id"
                },
                "as": "progress",
                "pipeline": [
                    {
                        "$match": {
                            "$expr": {
                                "$in": ["$$task_id", "$actual.task_id"]
                            }
                        }
                    },
                    {
                        "$unwind": "$actual"
                    },
                    {
                        "$match": {
                            "$expr": {
                                "$eq": ["$$task_id", "$actual.task_id"]
                            }
                        }
                    },
                    {
                        "$group": {
                            "_id": "$actual.task_id",
                            "value": {
                                "$sum": "$actual.value"
                            }
                        }
                    }
                ]
            }
        });
        pipeline.push(doc! {
            "$lookup": {
                "from": "project-reports",
                "let": {
                    "task_id": "$_id"
                },
                "as": "actual",
                "pipeline": [
                    {
                        "$match": {
                            "$expr": {
                                "$in": ["$$task_id", "$actual.task_id"]
                            }
                        }
                    },
                    {
                        "$unwind": "$actual"
                    },
                    {
                        "$match": {
                            "$expr": {
                                "$eq": ["$$task_id", "$actual.task_id"]
                            }
                        }
                    },
                    {
                        "$group": {
                            "_id": "$actual.task_id",
                            "start": {
                                "$min": {
                                    "$toLong": "$date"
                                }
                            },
                            "end": {
                                "$max": {
                                    "$toLong": "$date"
                                }
                            }
                        }
                    }
                ]
            }
        });

        if query.subtask {
            pipeline.push(doc! {
                "$lookup": {
                    "from": "project-tasks",
                    "let": {
                        "task_id": "$_id"
                    },
                    "as": "task",
                    "pipeline": [
                        {
                            "$match": {
                                "$expr": {
                                    "$eq": ["$task_id", "$$task_id"]
                                }
                            }
                        },
                        {
                            "$project": {
                                "_id": {
                                    "$toString": "$_id"
                                },
                                "name": "$name",
                                "period": {
                                    "$cond": [
                                        "$period",
                                        {
                                            "start": {
                                                "$toString": "$period.start"
                                            },
                                            "end": {
                                                "$toString": "$period.end"
                                            },
                                        },
                                        to_bson::<Option<ObjectId>>(&None).unwrap()
                                    ]
                                },
                                "status": "$status",
                                "volume": "$volume",
                            }
                        }
                    ]
                }
            })
        }

        pipeline.push(doc! {
            "$project": {
                "_id": {
                    "$toString": "$_id"
                },
                "task_id": "$task_id",
                "user": "$user",
                "task": "$task",
                "name": "$name",
                "period": {
                    "$cond": [
                        "$period",
                        {
                            "start": {
                                "$toString": "$period.start"
                            },
                            "end": {
                                "$toString": "$period.end"
                            },
                        },
                        to_bson::<Option<ObjectId>>(&None).unwrap()
                    ]
                },
                "actual": {
                    "$cond": [
                        {
                            "$gt": [
                                {
                                    "$size": "$actual"
                                },
                                0
                            ]
                        },
                        {
                            "start": {
                                "$toString": {
                                    "$toDate": {
                                        "$first": "$actual.start"
                                    }
                                }
                            },
                            "end": {
                                "$cond": [
                                    {
                                        "$or": [
                                            {
                                                "$eq": [
                                                    {
                                                        "$first": "$status.kind"
                                                    },
                                                    to_bson::<ProjectTaskStatusKind>(&ProjectTaskStatusKind::Running).unwrap()
                                                ]
                                            },
                                            {
                                                "$eq": [
                                                    {
                                                        "$first": "$status.kind"
                                                    },
                                                    to_bson::<ProjectTaskStatusKind>(&ProjectTaskStatusKind::Paused).unwrap()
                                                ]
                                            },
                                        ]
                                    },
                                    {
                                        "$toString": {
                                            "$toDate": Utc::now().timestamp_millis()
                                        }
                                    },
                                    {
                                        "$toString": {
                                            "$toDate": {
                                                "$first": "$actual.end"
                                            }
                                        }
                                    }
                                ]
                            },
                        },
                        to_bson::<Option<ObjectId>>(&None).unwrap()
                    ]
                },
                "status": "$status",
                "volume": "$volume",
                "value": "$value",
                "progress": {
                    "$cond": [
                        {
                            "$gt": [
                                { "$size": "$progress" },
                                0
                            ]
                        },
                        {
                            "$first": "$progress.value"
                        },
                        0.0
                    ]
                }
            }
        });

        if let Ok(mut cursor) = collection.aggregate(pipeline, None).await {
            let mut tasks: Vec<ProjectTaskMinResponse> = Vec::<ProjectTaskMinResponse>::new();
            while let Some(Ok(doc)) = cursor.next().await {
                let task: ProjectTaskMinResponse =
                    from_document::<ProjectTaskMinResponse>(doc).unwrap();
                tasks.push(task);
            }
            if !tasks.is_empty() {
                if !dependencies.is_empty() {
                    for task in tasks.iter_mut() {
                        let mut _id = task.task_id;
                        let mut found = true;
                        while found {
                            if let Some(task_id) = _id {
                                if let Some(index) =
                                    dependencies.iter().position(|a| a._id.unwrap() == task_id)
                                {
                                    task.value *= dependencies[index].value / 100.0;
                                    _id = dependencies[index].task_id;
                                }
                            } else {
                                found = false;
                            }
                        }
                    }
                }

                Ok(Some(tasks))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }
    pub async fn find_many_area(
        project_id: &ObjectId,
    ) -> Result<Option<Vec<ProjectAreaResponse>>, String> {
        let db: Database = get_db();
        let collection: Collection<Project> = db.collection::<Project>("projects");

        let pipeline: Vec<mongodb::bson::Document> = vec![
            doc! {
                "$match": {
                    "$expr": {
                        "$eq": [ "$_id", to_bson::<ObjectId>(project_id).unwrap() ]
                    }
                }
            },
            doc! {
                "$lookup": {
                    "from": "project-tasks",
                    "let": {
                        "project_id": "$_id",
                        "member": "$member"
                    },
                    "as": "tasks",
                    "pipeline": [
                        {
                            "$match": {
                                "$expr": {
                                    "$and": [
                                        {
                                            "$eq": ["$project_id", "$$project_id"]
                                        },
                                        {
                                            "$eq": ["$task_id", to_bson::<Option<ObjectId>>(&None).unwrap()]
                                        }
                                    ]
                                }
                            }
                        },
                        {
                            "$lookup": {
                                "from": "users",
                                "as": "users",
                                "let": {
                                    "user_id": {
                                        "$map": {
                                            "input": {
                                                "$filter": {
                                                    "input": "$$member",
                                                    "cond": {
                                                        "$and": [
                                                            {
                                                                "$ne": ["$$this.kind", "support"]
                                                            },
                                                            {
                                                                "$in": [
                                                                    "$$this._id",
                                                                    {
                                                                        "$cond": [
                                                                            "$user_id",
                                                                            "$user_id",
                                                                            []
                                                                        ]
                                                                    }
                                                                ]
                                                            }
                                                        ]
                                                    }
                                                }
                                            },
                                            "in": "$$this._id"
                                        }
                                    }
                                },
                                "pipeline": [
                                    {
                                        "$match": {
                                            "$expr": {
                                                "$in": ["$_id", "$$user_id"]
                                            }
                                        },
                                    },
                                    {
                                        "$project": {
                                            "_id": {
                                                "$toString": "$_id"
                                            },
                                            "name": "$name",
                                            "image": "$image",
                                        }
                                    }
                                ]
                            }
                        },
                        {
                            "$lookup": {
                                "from": "project-tasks",
                                "let": {
                                    "task_id": "$_id"
                                },
                                "as": "task",
                                "pipeline": [
                                    {
                                        "$match": {
                                            "$expr": {
                                                "$eq": ["$task_id", "$$task_id"]
                                            }
                                        }
                                    },
                                    {
                                        "$project": {
                                            "_id": {
                                                "$toString": "$_id"
                                            },
                                            "name": "$name",
                                            "period": {
                                                "$cond": [
                                                    "$period",
                                                    {
                                                        "start": {
                                                            "$toString": "$period.start"
                                                        },
                                                        "end": {
                                                            "$toString": "$period.end"
                                                        },
                                                    },
                                                    to_bson::<Option<ObjectId>>(&None).unwrap()
                                                ]
                                            },
                                            "status": "$status",
                                            "volume": "$volume",
                                        }
                                    }
                                ]
                            }
                        },
                        {
                            "$lookup": {
                                "from": "project-reports",
                                "let": {
                                    "task_id": "$_id"
                                },
                                "as": "progress",
                                "pipeline": [
                                    {
                                        "$match": {
                                            "$expr": {
                                                "$in": ["$$task_id", "$actual.task_id"]
                                            }
                                        }
                                    },
                                    {
                                        "$unwind": "$actual"
                                    },
                                    {
                                        "$match": {
                                            "$expr": {
                                                "$eq": ["$$task_id", "$actual.task_id"]
                                            }
                                        }
                                    },
                                    {
                                        "$group": {
                                            "_id": "$actual.task_id",
                                            "value": {
                                                "$sum": "$actual.value"
                                            }
                                        }
                                    }
                                ]
                            }
                        },
                        {
                            "$project": {
                                "_id": {
                                    "$toString": "$_id"
                                },
                                "area_id": "$area_id",
                                "task_id": "$task_id",
                                "user": {
                                    "$concatArrays": [
                                        "$users",
                                        {
                                            "$map": {
                                                "input": {
                                                    "$filter": {
                                                        "input": "$$member",
                                                        "cond": {
                                                            "$and": [
                                                                {
                                                                    "$eq": ["$$this.kind", "support"]
                                                                },
                                                                {
                                                                    "$in": [
                                                                        "$$this._id",
                                                                        {
                                                                            "$cond": [
                                                                                "$user_id",
                                                                                "$user_id",
                                                                                []
                                                                            ]
                                                                        }
                                                                    ]
                                                                }
                                                            ]
                                                        }
                                                    }
                                                },
                                                "in": {
                                                    "_id": {
                                                        "$toString": "$$this._id"
                                                    },
                                                    "name": "$$this.name"
                                                }
                                            }
                                        }
                                    ]
                                },
                                "task": "$task",
                                "name": "$name",
                                "period": {
                                    "$cond": [
                                        "$period",
                                        {
                                            "start": {
                                                "$toString": "$period.start"
                                            },
                                            "end": {
                                                "$toString": "$period.end"
                                            },
                                        },
                                        to_bson::<Option<ObjectId>>(&None).unwrap()
                                    ]
                                },
                                "status": "$status",
                                "volume": "$volume",
                                "value": "$value",
                                "progress": {
                                    "$cond": [
                                        {
                                            "$gt": [
                                                { "$size": "$progress" },
                                                0
                                            ]
                                        },
                                        {
                                            "$first": "$progress.value"
                                        },
                                        0.0
                                    ]
                                }
                            }
                        },
                    ]
                }
            },
            doc! {
                "$project": {
                    "area": {
                        "$map": {
                            "input": "$area",
                            "in": {
                                "_id": {
                                    "$toString": "$$this._id"
                                },
                                "name": "$$this.name",
                                "task": {
                                    "$filter": {
                                        "input": "$tasks",
                                        "as": "task",
                                        "cond": {
                                            "$eq": ["$$this._id", "$$task.area_id"]
                                        }
                                    }
                                }
                            }
                        }
                    },
                }
            },
            doc! {
                "$unwind": "$area"
            },
            doc! {
                "$project": {
                    "_id": "$area._id",
                    "name": "$area.name",
                    "task": "$area.task",
                }
            },
        ];
        let mut areas: Vec<ProjectAreaResponse> = Vec::new();

        match collection.aggregate(pipeline, None).await {
            Ok(mut cursor) => {
                while let Some(Ok(doc)) = cursor.next().await {
                    let area: ProjectAreaResponse =
                        from_document::<ProjectAreaResponse>(doc).unwrap();
                    areas.push(area);
                }
                if !areas.is_empty() {
                    Ok(Some(areas))
                } else {
                    Ok(None)
                }
            }
            Err(err) => {
                println!("{:#?}", err);
                Err("PROJECT_TASK_NOT_FOUND".to_string())
            }
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
                            "$project": {
                                "_id": {
                                    "$toString": "$_id"
                                },
                                "name": "$name",
                                "period": {
                                    "$cond": [
                                        { "$ne": ["$period", to_bson::<Option<ObjectId>>(&None).unwrap()] },
                                        {
                                            "start": { "$toString": "$period.start" },
                                            "end": { "$toString": "$period.end" },
                                        },
                                        to_bson::<Option<ObjectId>>(&None).unwrap()
                                    ]
                                },
                                "status": "$status",
                                "volume": "$volume"
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
                                "_id": {
                                    "$toString": "$_id"
                                },
                                "name": "$name",
                                "member": "$member",
                                "area": {
                                    "$first": {
                                        "$filter": {
                                            "input": "$area",
                                            "cond": {
                                                "$eq": ["$$this._id", "$$area_id"]
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    ]
                }
            },
            doc! {
                "$lookup": {
                    "from": "users",
                    "as": "users",
                    "let": {
                        "user_id": {
                            "$map": {
                                "input": {
                                    "$filter": {
                                        "input": {
                                            "$first": "$project.member"
                                        },
                                        "cond": {
                                            "$and": [
                                                {
                                                    "$ne": ["$$this.kind", "support"]
                                                },
                                                {
                                                    "$in": [
                                                        "$$this._id",
                                                        {
                                                            "$cond": [
                                                                "$user_id",
                                                                "$user_id",
                                                                []
                                                            ]
                                                        }
                                                    ]
                                                }
                                            ]
                                        }
                                    }
                                },
                                "in": "$$this._id"
                            }
                        }
                    },
                    "pipeline": [
                        {
                            "$match": {
                                "$expr": {
                                    "$in": ["$_id", "$$user_id"]
                                }
                            },
                        },
                        {
                            "$project": {
                                "_id": {
                                    "$toString": "$_id"
                                },
                                "name": "$name",
                                "image": "$image",
                            }
                        }
                    ]
                }
            },
            doc! {
                "$project": {
                    "_id": {
                        "$toString": "$_id"
                    },
                    "project": {
                        "$first": "$project"
                    },
                    "area": {
                        "_id": {
                            "$toString": {
                                "$first": "$project.area._id"
                            }
                        },
                        "name": {
                            "$first": "$project.area.name"
                        },
                    },
                    "user": {
                        "$concatArrays": [
                            "$users",
                            {
                                "$map": {
                                    "input": {
                                        "$filter": {
                                            "input": {
                                                "$first": "$project.member"
                                            },
                                            "cond": {
                                                "$and": [
                                                    {
                                                        "$eq": ["$$this.kind", "support"]
                                                    },
                                                    {
                                                        "$in": [
                                                            "$$this._id",
                                                            {
                                                                "$cond": [
                                                                    "$user_id",
                                                                    "$user_id",
                                                                    []
                                                                ]
                                                            }
                                                        ]
                                                    }
                                                ]
                                            }
                                        }
                                    },
                                    "in": {
                                        "_id": {
                                            "$toString": "$$this._id"
                                        },
                                        "name": "$$this.name"
                                    }
                                }
                            }
                        ]
                    },
                    "task": to_bson::<Option<ObjectId>>(&None).unwrap(),
                    "name": "$name",
                    "description": "$description",
                    "period": {
                        "$cond": [
                            { "$ne": ["$period", to_bson::<Option<ObjectId>>(&None).unwrap()] },
                            {
                                "start": { "$toString": "$period.start" },
                                "end": { "$toString": "$period.end" },
                            },
                            to_bson::<Option<ObjectId>>(&None).unwrap()
                        ]
                    },
                    "status": "$status",
                    "volume": "$volume",
                    "value": "$value",
                    "progress": {
                        "$cond": [
                            {
                                "$gt": [
                                    { "$size": "$report" },
                                    0
                                ]
                            },
                            {
                                "$first": "$report.progress"
                            },
                            0
                        ]
                    },
                }
            },
        ];

        if let Ok(mut cursor) = collection.aggregate(pipeline, None).await {
            if let Some(Ok(doc)) = cursor.next().await {
                let mut task = from_document::<ProjectTaskResponse>(doc).unwrap();
                task.task = Self::find_many_timeline(&ProjectTaskTimelineQuery {
                    project_id: task.project._id.parse::<ObjectId>().unwrap(),
                    area_id: None,
                    task_id: Some(*_id),
                    status: None,
                    relative: true,
                    subtask: true,
                })
                .await
                .map_or_else(|_| Some(Vec::<ProjectTaskMinResponse>::new()), |task| task);
                Ok(Some(task))
            } else {
                Ok(None)
            }
        } else {
            Err("PROJECT_TASK_NOT_FOUND".to_string())
        }
    }
}
