use crate::database::get_db;

use actix_multipart::form::{tempfile::TempFile, MultipartForm};
use futures::stream::StreamExt;
use mongodb::{
    bson::{doc, from_document, oid::ObjectId, to_bson, DateTime, Document},
    Collection, Database,
};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

use super::{
    project::{Project, ProjectMemberResponse, ProjectStatusKind},
    project_task::{ProjectTask, ProjectTaskQuery, ProjectTaskQueryKind, ProjectTaskStatusKind},
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
    pub user_id: ObjectId,
    pub member_id: Option<Vec<ObjectId>>,
    pub date: DateTime,
    pub time: Option<[[usize; 2]; 2]>,
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
    pub _id: ObjectId,
    pub description: Option<String>,
    pub extension: String,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct ProjectProgressReportWeather {
    pub time: [usize; 2],
    pub kind: ProjectProgressReportWeatherKind,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct ProjectProgressReportDocumentationRequest {
    pub description: Option<String>,
    pub extension: String,
}

pub struct ProjectProgressReportQuery {
    pub project_id: ObjectId,
    pub area_id: Option<ObjectId>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ProjectProgressReportRequest {
    pub member_id: Option<Vec<ObjectId>>,
    pub time: Option<[[usize; 2]; 2]>,
    pub actual: Option<Vec<ProjectProgressReportActual>>,
    pub plan: Option<Vec<ProjectProgressReportPlan>>,
    pub weather: Option<Vec<ProjectProgressReportWeather>>,
    pub documentation: Option<Vec<ProjectProgressReportDocumentationRequest>>,
}
#[derive(Debug, MultipartForm)]
pub struct ProjectProgressReportDocumentationMultipartRequest {
    #[multipart(rename = "file")]
    pub files: Vec<TempFile>,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct ProjectProgressReportResponse {
    pub _id: String,
    pub user: ProjectProgressReportUserResponse,
    pub project: ProjectProgressReportProjectResponse,
    pub date: String,
    pub time: Option<[[usize; 2]; 2]>,
    pub member: Option<Vec<ProjectMemberResponse>>,
    pub actual: Option<Vec<ProjectProgressReportActualResponse>>,
    pub plan: Option<Vec<ProjectProgressReportPlanResponse>>,
    pub weather: Option<Vec<ProjectProgressReportWeather>>,
    pub documentation: Option<Vec<ProjectProgressReportDocumentationResponse>>,
    pub progress: f64,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct ProjectProgressReportMinResponse {
    pub _id: String,
    pub user: ProjectProgressReportUserResponse,
    pub project: ProjectProgressReportProjectResponse,
    pub date: String,
    pub time: Option<[[usize; 2]; 2]>,
    pub member: Option<Vec<ProjectMemberResponse>>,
    pub actual: Option<Vec<ProjectProgressReportActual>>,
    pub plan: Option<Vec<ProjectProgressReportPlan>>,
    pub weather: Option<Vec<ProjectProgressReportWeather>>,
    pub documentation: Option<Vec<ProjectProgressReportDocumentation>>,
    pub progress: f64,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct ProjectProgressReportUserResponse {
    pub _id: String,
    pub name: String,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct ProjectProgressReportProjectResponse {
    pub _id: String,
    pub name: String,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct ProjectProgressReportPlanResponse {
    pub _id: String,
    pub name: String,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct ProjectProgressReportActualResponse {
    pub _id: String,
    pub name: String,
    pub value: f64,
    pub area: ProjectProgressReportActualAreaResponse,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct ProjectProgressReportActualAreaResponse {
    pub _id: String,
    pub name: String,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct ProjectProgressReportDocumentationResponse {
    pub _id: String,
    pub description: Option<String>,
    pub extension: String,
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

        if let Some(time) = self.time {
            let (start_time, end_time) = (time[0], time[1]);
            if start_time[0] > 23
                || start_time[1] > 59
                || end_time[0] > 23
                || end_time[1] > 59
                || (start_time[0] * 60 + start_time[1]) >= (end_time[0] * 60 + end_time[1])
            {
                return Err("PROJECT_REPORT_TIME_INVALID".to_string());
            }
        }

        if let Some(actual) = self.actual.as_mut() {
            let mut invalid_task_index = Vec::<usize>::new();
            if project.status.get(0).unwrap().kind == ProjectStatusKind::Pending
                || project.status.get(0).unwrap().kind == ProjectStatusKind::Paused
            {
                project
                    .update_status(ProjectStatusKind::Running, None)
                    .await
                    .map_err(|_| "PROJECT_UPDATE_FAILED".to_string())?;
            }
            for (i, actual_task) in actual.iter_mut().enumerate() {
                if let Ok(Some(task)) = ProjectTask::find_detail_by_id(&actual_task.task_id).await {
                    if task.task.is_some() {
                        invalid_task_index.push(i);
                        continue;
                    }
                    let remain = 100.0 - task.progress;
                    if (remain - actual_task.value).abs() <= 0.001 {
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
                        if status.kind != ProjectTaskStatusKind::Running {
                            let mut task = ProjectTask::find_by_id(&actual_task.task_id)
                                .await
                                .map_err(|_| "PROJECT_TASK_NOT_FOUND".to_string())?
                                .ok_or_else(|| "PROJECT_TASK_NOT_FOUND".to_string())?;
                            task.update_status(ProjectTaskStatusKind::Running, None)
                                .await
                                .map_err(|_| "PROJECT_TASK_UPDATE_FAILED".to_string())?;
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
    pub async fn find_many(
        query: ProjectProgressReportQuery,
    ) -> Result<Option<Vec<ProjectProgressReport>>, String> {
        let db: Database = get_db();
        let collection: Collection<ProjectProgressReport> =
            db.collection::<ProjectProgressReport>("project-reports");

        let mut pipeline: Vec<Document> = Vec::<Document>::new();
        let mut queries: Vec<Document> = Vec::<Document>::new();

        queries.push(doc! {
            "$eq": [ "$project_id", to_bson::<ObjectId>(&query.project_id).unwrap() ]
        });

        pipeline.push(doc! {
            "$match": {
                "$expr": {
                    "$and": queries
                }
            }
        });

        if let Ok(mut cursor) = collection.aggregate(pipeline, None).await {
            let mut reports: Vec<ProjectProgressReport> = Vec::<ProjectProgressReport>::new();
            while let Some(Ok(doc)) = cursor.next().await {
                let report: ProjectProgressReport =
                    from_document::<ProjectProgressReport>(doc).unwrap();
                reports.push(report);
            }
            if !reports.is_empty() {
                Ok(Some(reports))
            } else {
                Ok(None)
            }
        } else {
            Err("PROJECT_TASK_NOT_FOUND".to_string())
        }
    }
    pub async fn find_detail_by_id(
        _id: &ObjectId,
    ) -> Result<Option<ProjectProgressReportResponse>, String> {
        let db: Database = get_db();
        let collection: Collection<ProjectProgressReport> =
            db.collection::<ProjectProgressReport>("project-reports");

        let pipeline = vec![
            doc! {
                "$match": {
                    "$expr": {
                        "$eq": ["$_id", to_bson::<ObjectId>(_id).unwrap()]
                    }
                }
            },
            doc! {
                "$lookup": {
                    "from": "projects",
                    "as": "project",
                    "let": {
                        "project_id": "$project_id"
                    },
                    "pipeline": [
                        {
                            "$match": {
                                "$expr": {
                                    "$eq": ["$_id", "$$project_id"]
                                }
                            }
                        }
                    ]
                }
            },
            doc! {
                "$lookup": {
                    "from": "users",
                    "as": "user",
                    "let": {
                        "user_id": "$user_id"
                    },
                    "pipeline": [
                        {
                            "$match": {
                                "$expr": {
                                    "$eq": ["$_id", "$$user_id"]
                                }
                            }
                        },
                        {
                            "$project": {
                                "_id": {
                                    "$toString": "$_id"
                                },
                                "name": "$name"
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
                        "user": {
                            "$cond": [
                                "$member_id",
                                {
                                    "$map": {
                                        "input": {
                                            "$filter": {
                                                "input": {
                                                    "$filter": {
                                                        "input": { "$first": "$project.member" },
                                                        "cond": {
                                                            "$ne": ["$$this.kind", "support"]
                                                        }
                                                    }
                                                },
                                                "cond": {
                                                    "$in": [
                                                        "$$this._id",
                                                        {
                                                            "$cond": [
                                                                "$member_id",
                                                                "$member_id",
                                                                []
                                                            ]
                                                        }
                                                    ]
                                                }
                                            }
                                        },
                                        "in": {
                                            "_id": "$$this._id",
                                            "kind": "$$this.kind",
                                            "role_id": "$$this.role_id"
                                        }
                                    }
                                },
                                {
                                    "_id": []
                                }
                            ]
                        }
                    },
                    "pipeline": [
                        {
                            "$match": {
                                "$expr": {
                                    "$in": ["$_id", "$$user._id"]
                                }
                            }
                        },
                        {
                            "$project": {
                                "role_id": {
                                    "$arrayElemAt": [
                                        "$$user.role_id",
                                        {
                                            "$indexOfArray": ["$$user._id", "$_id"]
                                        }
                                    ]
                                },
                                "kind": {
                                    "$arrayElemAt": [
                                        "$$user.kind",
                                        {
                                            "$indexOfArray": ["$$user._id", "$_id"]
                                        }
                                    ]
                                },
                                "name": "$name",
                                "image": "$image"
                            }
                        }
                    ]
                }
            },
            doc! {
                "$project": {
                    "user": {
                        "$first": "$user"
                    },
                    "project": {
                        "_id": {
                            "$toString": {
                                "$first": "$project._id"
                            }
                        },
                        "name": {
                            "$first": "$project.name"
                        },
                        "area": {
                            "$first": "$project.area"
                        }
                    },
                    "date": { "$toString": "$date" },
                    "time": "$time",
                    "member": {
                        "$concatArrays": [
                            "$users",
                            {
                                "$map": {
                                    "input": {
                                        "$filter": {
                                            "input": {
                                                "$filter": {
                                                    "input": {
                                                        "$first": "$project.member"
                                                    },
                                                    "cond": {
                                                        "$eq": ["$$this.kind", "support"]
                                                    }
                                                }
                                            },
                                            "cond": {
                                                "$in": [
                                                    "$$this._id",
                                                    {
                                                        "$cond": [
                                                            "$member_id",
                                                            "$member_id",
                                                            []
                                                        ]
                                                    }
                                                ]
                                            }
                                        }
                                    },
                                    "in": {
                                        "_id": "$$this._id",
                                        "name": "$$this.name",
                                        "kind": "$$this.kind",
                                        "role_id": "$$this.role_id"
                                    }
                                }
                            }
                        ]
                    },
                    "actual": "$actual",
                    "plan": "$plan",
                    "weather": "$weather",
                    "documentation": "$documentation",
                }
            },
            doc! {
                "$lookup": {
                    "from": "project-roles",
                    "as": "roles",
                    "let": {
                        "role_id": {
                            "$reduce": {
                                "input": "$member.role_id",
                                "initialValue": [],
                                "in": {
                                    "$concatArrays": [
                                        "$$value",
                                        {
                                            "$filter": {
                                                "input": "$$this",
                                                "as": "role_id",
                                                "cond": {
                                                    "$ne": [
                                                        {
                                                            "$in": ["$$role_id", "$$value"]
                                                        },
                                                        to_bson::<bool>(&true).unwrap()
                                                    ]
                                                }
                                            }
                                        }
                                    ]
                                }
                            }
                        }
                    },
                    "pipeline": [
                        {
                            "$match": {
                                "$expr": {
                                    "$in": ["$_id", "$$role_id"]
                                }
                            }
                        },
                        {
                            "$project": {
                                "name": "$name",
                                "permission": "$permission",
                            }
                        }
                    ]
                }
            },
            doc! {
                "$lookup": {
                    "from": "project-tasks",
                    "as": "actual",
                    "let": {
                        "actual": "$actual",
                        "area": {
                            "$first": "$project.area"
                        }
                    },
                    "pipeline": [
                        {
                            "$match": {
                                "$expr": {
                                    "$in": ["$_id", "$$actual.task_id"]
                                }
                            }
                        },
                        {
                            "$project": {
                                "_id": {
                                    "$toString": "$_id"
                                },
                                "area": {
                                    "_id": {
                                        "$toString": "$area_id"
                                    },
                                    "name": {
                                        "$arrayElemAt": [
                                            "$$area.name",
                                            {
                                                "$indexOfArray": ["$$area._id", "$area_id"]
                                            }
                                        ]
                                    }
                                },
                                "name": "$name",
                                "value": {
                                    "$arrayElemAt": [
                                        "$$actual.value",
                                        {
                                            "$indexOfArray": [
                                                "$$actual._id",
                                                "$_id"
                                            ]
                                        }
                                    ]
                                }
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
                    "user": "$user",
                    "project": {
                        "$first": "$project"
                    },
                    "date": "$date",
                    "time": "$time",
                    "member": {
                        "$map": {
                            "input": "$member",
                            "in": {
                                "_id": { "$toString": "$$this._id" },
                                "name": "$$this.name",
                                "kind": "$$this.kind",
                                "image": "$$this.image",
                                "role": {
                                    "$map": {
                                        "input": "$$this.role_id",
                                        "as": "role_id",
                                        "in": {
                                            "_id": {
                                                "$toString": "$$role_id"
                                            },
                                            "name": {
                                                "$arrayElemAt": [
                                                    "$roles.name",
                                                    {
                                                        "$indexOfArray": [
                                                            "$roles._id",
                                                            "$$role_id"
                                                        ]
                                                    }
                                                ]
                                            },
                                            "permission": {
                                                "$arrayElemAt": [
                                                    "$roles.permission",
                                                    {
                                                        "$indexOfArray": [
                                                            "$roles._id",
                                                            "$$role_id"
                                                        ]
                                                    }
                                                ]
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    },
                    "actual": "$actual",
                    "plan": "$plan",
                    "weather": "$weather",
                    "documentation": {
                        "$map": {
                            "input": "$documentation",
                            "in": {
                                "_id": { "$toString": "$$this._id" },
                                "extension": "$$this.extension",
                                "description": "$$this.description",
                            }
                        }
                    },
                }
            },
            doc! {
                "$addFields": {
                    "progress": to_bson::<f64>(&0.0).unwrap(),
                }
            },
        ];
        let mut dependencies: Vec<ProjectTask> = Vec::new();

        if let Ok(Some(tasks)) = ProjectTask::find_many(&ProjectTaskQuery {
            _id: None,
            project_id: Some(*_id),
            task_id: None,
            area_id: None,
            limit: None,
            kind: Some(ProjectTaskQueryKind::Dependency),
        })
        .await
        {
            dependencies = tasks;
        }

        if let Ok(mut cursor) = collection.aggregate(pipeline, None).await {
            if let Some(Ok(doc)) = cursor.next().await {
                let mut report = from_document::<ProjectProgressReportResponse>(doc).unwrap();
                if let Some(tasks) = &report.actual {
                    for task in tasks.iter() {
                        if let Ok(Some(base)) =
                            ProjectTask::find_by_id(&ObjectId::from_str(&task._id).unwrap()).await
                        {
                            let mut _id = base.task_id;
                            let mut found = true;
                            let mut count = task.value * base.value / 100.0;

                            while found {
                                if let Some(task_id) = _id {
                                    if let Some(index) =
                                        dependencies.iter().position(|a| a._id.unwrap() == task_id)
                                    {
                                        count *= dependencies[index].value / 100.0;
                                        _id = dependencies[index].task_id;
                                    } else {
                                        found = false;
                                    }
                                } else {
                                    found = false;
                                }
                            }

                            report.progress += count;
                        }
                    }
                }
                Ok(Some(report))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
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
