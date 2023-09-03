use crate::{
    database::get_db,
    models::{
        project::{
            Project, ProjectCustomerImageResponse, ProjectCustomerResponse, ProjectPeriodResponse,
            ProjectProgressResponse,
        },
        project_task::{ProjectTask, ProjectTaskQuery, ProjectTaskQueryKind},
    },
};
use actix_web::{get, web, HttpResponse};
use futures::stream::StreamExt;
use mime_guess::from_path;
use mongodb::bson::{doc, from_document, oid::ObjectId, to_bson};
use serde::{Deserialize, Serialize};
use std::fs;

use crate::models::project_task::{ProjectTaskAreaResponse, ProjectTaskPeriodResponse};

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FileKind {
    ProjectDocumentation,
    CompanyImage,
    CustomerImage,
    UserImage,
}
#[derive(Deserialize)]
pub struct FileQueryParams {
    pub kind: FileKind,
    pub name: String,
}
#[derive(Deserialize, Debug)]
pub struct OverviewCount {
    pub project_count: usize,
    pub project_completed: usize,
    pub project_completition: f64,
}
#[derive(Serialize)]
pub struct Overview {
    pub project_count: usize,
    pub project_completed: usize,
    pub project_completition: f64,
    pub project: Vec<OverviewProject>,
    pub task: Vec<OverviewTask>,
}
#[derive(Deserialize, Serialize, Clone)]
pub struct OverviewProject {
    pub _id: String,
    pub customer: ProjectCustomerResponse,
    pub name: String,
    pub code: String,
    pub period: ProjectPeriodResponse,
    pub progress: Option<ProjectProgressResponse>,
}
#[derive(Deserialize, Serialize)]
pub struct OverviewTask {
    pub _id: String,
    pub project: OverviewProject,
    pub area: ProjectTaskAreaResponse,
    pub name: String,
    pub description: Option<String>,
    pub period: Option<ProjectTaskPeriodResponse>,
}

pub mod company;
pub mod customer;
pub mod project;
pub mod role;
pub mod user;

#[get("/files")]
pub async fn get_file(query: web::Query<FileQueryParams>) -> HttpResponse {
    let path = match query.kind {
        FileKind::ProjectDocumentation => format!("./files/reports/documentation/{}", query.name),
        FileKind::CompanyImage => format!("./files/companies/{}", query.name),
        FileKind::CustomerImage => format!("./files/customers/{}", query.name),
        FileKind::UserImage => format!("./files/users/{}", query.name),
    };
    if let Ok(file) = fs::read(path.clone()) {
        let mime = from_path(path).first_or_octet_stream();
        HttpResponse::Ok().content_type(mime).body(file)
    } else {
        HttpResponse::NotFound().body("CONTENT_NOT_FOUND")
    }
}
#[get("/overview")]
pub async fn get_overview() -> HttpResponse {
    let db = get_db();
    let collection = db.collection::<ProjectTask>("project-tasks");

    let mut overview = Overview {
        project_count: 0,
        project_completed: 0,
        project_completition: 0.0,
        project: Vec::new(),
        task: Vec::new(),
    };
    let mut task_id = Vec::<ObjectId>::new();

    if let Ok(Some(tasks)) = ProjectTask::find_many(&ProjectTaskQuery {
        _id: None,
        project_id: None,
        task_id: None,
        area_id: None,
        limit: None,
        kind: Some(ProjectTaskQueryKind::Dependency),
    })
    .await
    {
        for task in tasks.iter() {
            if !task_id.contains(&task._id.unwrap()) {
                task_id.push(task._id.unwrap());
            }
        }
    }

    let pipeline = vec![
        doc! {
            "$match": {
                "$expr": {
                    "$and": [
                        {
                            "$eq": [
                                {
                                    "$first": "$status.kind"
                                },
                                "running"
                            ]
                        },
                        {
                            "$ne": [
                                {
                                    "$in": ["$_id", to_bson::<Vec<ObjectId>>(&task_id).unwrap()]
                                },
                                to_bson::<bool>(&true).unwrap()
                            ]
                        }
                    ]
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
                                "$eq": ["$$project_id", "$_id"]
                            }
                        }
                    },
                    {
                        "$lookup": {
                            "from": "customers",
                            "let": {
                                "customer_id": "$customer_id"
                            },
                            "as": "customer",
                            "pipeline": [
                                {
                                    "$match": {
                                        "$expr": {
                                            "$eq": ["$_id", "$$customer_id"]
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
                            "customer": {
                                "$first": "$customer"
                            },
                            "area": "$area",
                            "name": "$name",
                            "code": "$code",
                            "status": "$status",
                            "period": {
                                "start": { "$toString": "$period.start" },
                                "end": { "$toString": "$period.end" },
                            },
                            "progress": to_bson::<Option<ProjectProgressResponse>>(&None).unwrap()
                        }
                    },
                    {
                        "$project": {
                            "_id": {
                                "$toString": "$_id"
                            },
                            "customer": {
                                "_id": {
                                    "$toString": "$customer._id"
                                },
                                "name": "$customer.name",
                                "image": {
                                    "$cond": [
                                        "$customer.image",
                                        {
                                            "_id": {
                                                "$toString": "$customer.image._id"
                                            },
                                            "extension": "$customer.image.extension"
                                        },
                                        to_bson::<Option<ProjectCustomerImageResponse>>(&None).unwrap()
                                    ]
                                }
                            },
                            "area": "$area",
                            "name": "$name",
                            "code": "$code",
                            "status": "$status",
                            "period": "$period",
                            "progress": "$progress"
                        }
                    },
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
                    "$first": {
                        "$filter": {
                            "input": {
                                "$first": "$project.area"
                            },
                            "cond": {
                                "$eq": [
                                    "$area_id",
                                    "$$this._id"
                                ]
                            }
                        }
                    }
                },
                "task": "$task",
                "name": "$name",
                "description": "$description",
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
                }
            }
        },
        doc! {
            "$project": {
                "_id": "$_id",
                "project": "$project",
                "area": {
                    "_id": {
                        "$toString": "$area._id"
                    },
                    "name": "$area.name"
                },
                "task": "$task",
                "name": "$name",
                "description": "$description",
                "period": "$period"
            }
        },
    ];

    if let Ok(mut cursor) = collection.aggregate(pipeline, None).await {
        while let Some(Ok(doc)) = cursor.next().await {
            let task = from_document::<OverviewTask>(doc).unwrap();
            if overview
                .project
                .iter()
                .find(|a| &a._id == &task.project._id)
                .is_none()
            {
                let mut project = task.project.clone();
                project.progress =
                    Project::calculate_progress(&project._id.parse::<ObjectId>().unwrap())
                        .await
                        .map_or_else(|_| None, Some);
                overview.project.push(project);
            }
            overview.task.push(task);
        }

        let collection = db.collection::<ProjectTask>("projects");
        let pipeline = vec![doc! {
            "$group": {
                "_id": to_bson::<Option<ObjectId>>(&None).unwrap(),
                "project_count": {
                    "$sum": 1
                },
                "project_completed": {
                    "$sum": {
                        "$cond": [
                            {
                                "$eq": [
                                    {
                                        "$first": "$status.kind"
                                    },
                                    "finished"
                                ]
                            },
                            1,
                            0
                        ]
                    }
                },
                "project_completition": {
                    "$sum": {
                        "$cond": [
                            {
                                "$eq": [
                                    {
                                        "$first": "$status.kind"
                                    },
                                    "finished"
                                ]
                            },
                            100.0,
                            0.0
                        ]
                    }
                }
            }
        }];

        if let Ok(mut cursor) = collection.aggregate(pipeline, None).await {
            if let Some(Ok(doc)) = cursor.next().await {
                let count = from_document::<OverviewCount>(doc).unwrap();
                overview.project_count = count.project_count;
                overview.project_completed = count.project_completed;
                overview.project_completition = (count.project_completition
                    + overview.project.iter().fold(0.0, |a, b| {
                        a + (b.clone()).progress.map_or_else(|| 0.0, |v| v.actual)
                    }))
                    / (count.project_count as f64);
            }
        }
    }

    HttpResponse::Ok().json(overview)
}
