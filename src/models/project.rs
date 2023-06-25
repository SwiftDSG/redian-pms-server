use crate::database::get_db;

use chrono::Utc;
use futures::stream::StreamExt;
use mongodb::{
    bson::{doc, from_document, oid::ObjectId, to_bson, DateTime},
    Collection, Database,
};
use serde::{Deserialize, Serialize};

use super::{
    customer::Customer,
    project_progress_report::{ProjectProgressReport, ProjectProgressReportQuery},
    project_role::ProjectRoleResponse,
    project_task::{ProjectTask, ProjectTaskMinResponse, ProjectTaskQuery, ProjectTaskQueryKind},
    user::{User, UserImage},
};

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum ProjectMemberKind {
    Direct,
    Indirect,
    Support,
}
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProjectStatusKind {
    Running,
    Paused,
    Pending,
    Breakdown,
    Finished,
    Cancelled,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Project {
    pub _id: Option<ObjectId>,
    pub customer_id: ObjectId,
    pub name: String,
    pub code: String,
    pub period: ProjectPeriod,
    pub status: Vec<ProjectStatus>,
    pub area: Option<Vec<ProjectArea>>,
    pub member: Option<Vec<ProjectMember>>,
    pub leave: Option<Vec<DateTime>>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ProjectStatus {
    pub kind: ProjectStatusKind,
    pub time: DateTime,
    pub message: Option<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ProjectMember {
    pub _id: ObjectId,
    pub name: Option<String>,
    pub kind: ProjectMemberKind,
    pub role_id: Vec<ObjectId>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ProjectPeriod {
    pub start: DateTime,
    pub end: DateTime,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ProjectArea {
    pub _id: ObjectId,
    pub name: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ProjectAreaRequest {
    pub name: String,
}
#[derive(Debug)]
pub struct ProjectQuery {
    pub _id: Option<ObjectId>,
    pub limit: Option<usize>,
}
#[derive(Debug, Deserialize)]
pub struct ProjectRequest {
    pub customer_id: ObjectId,
    pub name: String,
    pub code: String,
    pub period: ProjectPeriodRequest,
    pub leave: Option<Vec<DateTime>>,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct ProjectMemberRequest {
    pub _id: Option<ObjectId>,
    pub name: Option<String>,
    pub kind: ProjectMemberKind,
    pub role_id: Vec<ObjectId>,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct ProjectPeriodRequest {
    pub start: i64,
    pub end: i64,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct ProjectMinResponse {
    pub _id: String,
    pub customer: ProjectCustomerResponse,
    pub name: String,
    pub code: String,
    pub period: ProjectPeriodResponse,
    pub status: Vec<ProjectStatus>,
    pub progress: Option<ProjectProgressResponse>,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct ProjectProgressResponse {
    pub plan: f64,
    pub actual: f64,
}
#[derive(Debug, Serialize)]
pub struct ProjectProgressGraphResponse {
    pub x: i64,
    pub y: Vec<f64>,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct ProjectResponse {
    pub _id: String,
    pub customer: ProjectCustomerResponse,
    pub name: String,
    pub code: String,
    pub period: ProjectPeriodResponse,
    pub status: Vec<ProjectStatus>,
    pub area: Option<Vec<ProjectArea>>,
    pub leave: Option<Vec<DateTime>>,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct ProjectCustomerResponse {
    pub _id: String,
    pub name: String,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct ProjectAreaResponse {
    pub _id: String,
    pub name: String,
    pub task: Option<Vec<ProjectTaskMinResponse>>,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct ProjectPeriodResponse {
    pub start: String,
    pub end: String,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct ProjectMemberResponse {
    pub _id: String,
    pub name: String,
    pub kind: ProjectMemberKind,
    pub role: Vec<ProjectRoleResponse>,
    pub image: Option<UserImage>,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct ProjectUserResponse {
    pub user: Option<Vec<ProjectMemberResponse>>,
    pub role: Option<Vec<ProjectRoleResponse>>,
}
impl Project {
    pub async fn save(&mut self) -> Result<ObjectId, String> {
        let db: Database = get_db();
        let collection: Collection<Project> = db.collection::<Project>("projects");

        self._id = Some(ObjectId::new());

        if let Ok(Some(_)) = Customer::find_by_id(&self.customer_id).await {
            collection
                .insert_one(self, None)
                .await
                .map_err(|_| "INSERTING_FAILED".to_string())
                .map(|result| result.inserted_id.as_object_id().unwrap())
        } else {
            Err("CUSTOMER_NOT_FOUND".to_string())
        }
    }
    pub async fn add_member(
        &mut self,
        members: &[ProjectMemberRequest],
    ) -> Result<ObjectId, String> {
        let db: Database = get_db();
        let collection: Collection<Project> = db.collection::<Project>("projects");

        let mut member: Vec<ProjectMember> = match &self.member {
            Some(member) => Vec::<ProjectMember>::from_iter(member.clone()),
            None => Vec::<ProjectMember>::new(),
        };

        for i in members.iter() {
            match i.kind {
                ProjectMemberKind::Support => {
                    if i.name.is_some() {
                        member.push(ProjectMember {
                            _id: ObjectId::new(),
                            name: i.name.clone(),
                            kind: i.kind.clone(),
                            role_id: i.role_id.clone(),
                        });
                    }
                }
                _ => {
                    if let Some(_id) = &i._id {
                        if (User::find_by_id(_id).await).is_ok() {
                            member.push(ProjectMember {
                                _id: _id.clone(),
                                name: None,
                                kind: i.kind.clone(),
                                role_id: i.role_id.clone(),
                            });
                        }
                    }
                }
            }
        }

        self.member = Some(member);

        collection
            .update_one(
                doc! { "_id": self._id.unwrap() },
                doc! { "$set": to_bson::<Project>(self).unwrap()},
                None,
            )
            .await
            .map_err(|_| "UPDATE_FAILED".to_string())
            .map(|_| self._id.unwrap())
    }
    pub async fn add_area(&mut self, areas: &[ProjectAreaRequest]) -> Result<ObjectId, String> {
        let db: Database = get_db();
        let collection: Collection<Project> = db.collection::<Project>("projects");

        let mut area: Vec<ProjectArea> = match &self.area {
            Some(area) => Vec::<ProjectArea>::from_iter(area.clone()),
            None => Vec::<ProjectArea>::new(),
        };

        for i in areas.iter() {
            let new_area = ProjectArea {
                _id: ObjectId::new(),
                name: i.name.clone(),
            };
            area.push(new_area);
        }

        self.area = Some(area);

        collection
            .update_one(
                doc! { "_id": self._id.unwrap() },
                doc! { "$set": to_bson::<Project>(self).unwrap()},
                None,
            )
            .await
            .map_err(|_| "UPDATE_FAILED".to_string())
            .map(|_| self._id.unwrap())
    }
    pub async fn calculate_progress(_id: &ObjectId) -> Result<ProjectProgressResponse, String> {
        let mut bases: Vec<ProjectTask> = Vec::new();
        let mut dependencies: Vec<ProjectTask> = Vec::new();
        let mut progresses: Vec<ProjectProgressReport> = Vec::new();

        if let Ok(Some(tasks)) = ProjectTask::find_many(&ProjectTaskQuery {
            _id: None,
            project_id: Some(*_id),
            task_id: None,
            area_id: None,
            limit: None,
            kind: Some(ProjectTaskQueryKind::Base),
        })
        .await
        {
            bases = tasks;
        }
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
        if let Ok(Some(reports)) = ProjectProgressReport::find_many(ProjectProgressReportQuery {
            project_id: *_id,
            area_id: None,
        })
        .await
        {
            progresses = reports;
        }

        if !bases.is_empty() && !dependencies.is_empty() {
            for task in bases.iter_mut() {
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

        let mut start_base = false;
        let mut start = 0;
        let end = Utc::now().timestamp_millis();

        if let Some(date) = bases
            .iter()
            .filter(|a| a.period.is_some())
            .map(|a| a.period.clone().unwrap().start.timestamp_millis())
            .min()
        {
            start = date;
            start_base = true;
        }
        if let Some(date) = progresses.iter().map(|a| a.date.timestamp_millis()).min() {
            if !start_base || date < start {
                start = date;
            }
        }

        let mut progress = ProjectProgressResponse {
            plan: 0.0,
            actual: 0.0,
        };
        if start != 0 {
            let diff = (end - start) / 86400000 + 1;
            println!("START DATE    : {:#?}", start);
            println!("END DATE      : {:#?}", end);
            println!("DAYS DIFF     : {:#?}", diff);
            println!("MILIS DIFF    : {:#?}", end - start);
            println!("=============================");
            for i in 0..diff {
                let date = start + i * 86400000;
                println!("DATE          : {:#?}", date);
                println!("DAYS          : {:#?}", date / 86400000);
                println!("=============================");
                let prev_plan = progress.plan;
                let prev_actual = progress.actual;
                let mut plan: f64 = bases
                    .iter()
                    .filter(|a| {
                        if let Some(period) = a.period.as_ref() {
                            let start = period.start.timestamp_millis();
                            let end = period.end.timestamp_millis();
                            date >= start && date <= end
                        } else {
                            false
                        }
                    })
                    .fold(prev_plan, |a, b| {
                        let period = b.period.as_ref().unwrap();
                        let start = period.start.timestamp_millis();
                        let end = period.end.timestamp_millis();
                        let diff = (end - start) / 86400000 + 1;
                        a + (b.value / (diff as f64))
                    });
                let mut actual = progresses
                    .iter()
                    .filter(|a| {
                        println!("FILTER DATE   : {:#?}", a.date.timestamp_millis());
                        println!(
                            "FILTER DAYS   : {:#?}",
                            a.date.timestamp_millis() / 86400000
                        );
                        println!("=============================");
                        date / 86400000 == a.date.timestamp_millis() / 86400000
                    })
                    .fold(prev_actual, |a, b| {
                        if let Some(actual) = &b.actual {
                            let progress = actual.iter().fold(0.0, |c, d| {
                                if let Some(index) =
                                    bases.iter().position(|e| e._id.unwrap() == d.task_id)
                                {
                                    c + d.value * bases[index].value / 100.0
                                } else {
                                    c
                                }
                            });
                            a + progress
                        } else {
                            a
                        }
                    });

                if plan >= 99.99 {
                    plan = 100.0
                }
                if actual >= 99.99 {
                    actual = 100.0
                }
                if plan == 100.0 && actual == 100.0 {
                    break;
                }

                progress = ProjectProgressResponse { plan, actual };
                println!("=============================\n\n");
            }
        }

        Ok(progress)
    }
    pub async fn find_many(query: &ProjectQuery) -> Result<Vec<ProjectMinResponse>, String> {
        let db: Database = get_db();
        let collection: Collection<Project> = db.collection::<Project>("projects");

        let mut pipeline: Vec<mongodb::bson::Document> = Vec::new();
        let mut projects: Vec<ProjectMinResponse> = Vec::new();

        if let Some(limit) = query.limit {
            pipeline.push(doc! {
                "$limit": to_bson::<usize>(&limit).unwrap()
            })
        }

        pipeline.push(doc! {
            "$lookup": {
                "from": "customers",
                "let": {
                    "customer_id": "$customer_id"
                },
                "as": "customers",
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
        });
        pipeline.push(doc! {
            "$project": {
                "_id": {
                    "$toString": "$_id"
                },
                "customer": {
                    "_id": {
                        "$toString": "$customer_id"
                    },
                    "name": {
                        "$first": "$customers.name"
                    }
                },
                "name": "$name",
                "code": "$code",
                "status": "$status",
                "period": {
                    "start": { "$toString": "$period.start" },
                    "end": { "$toString": "$period.end" },
                },
                "progress": to_bson::<Option<ProjectProgressResponse>>(&None).unwrap()
            }
        });

        if let Ok(mut cursor) = collection.aggregate(pipeline, None).await {
            while let Some(Ok(doc)) = cursor.next().await {
                let mut project: ProjectMinResponse =
                    from_document::<ProjectMinResponse>(doc).unwrap();
                project.progress =
                    Self::calculate_progress(&project._id.parse::<ObjectId>().unwrap())
                        .await
                        .map_or_else(|_| None, Some);
                projects.push(project);
            }
            if !projects.is_empty() {
                Ok(projects)
            } else {
                Err("PROJECT_NOT_FOUND".to_string())
            }
        } else {
            Err("PROJECT_NOT_FOUND".to_string())
        }
    }
    pub async fn find_by_id(_id: &ObjectId) -> Result<Option<Project>, String> {
        let db: Database = get_db();
        let collection: Collection<Project> = db.collection::<Project>("projects");

        collection
            .find_one(doc! { "_id": _id }, None)
            .await
            .map_err(|_| "PROJECT_NOT_FOUND".to_string())
    }
    pub async fn find_detail_by_id(_id: &ObjectId) -> Result<Option<ProjectResponse>, String> {
        let db: Database = get_db();
        let collection: Collection<Project> = db.collection::<Project>("projects");

        let pipeline: Vec<mongodb::bson::Document> = vec![
            doc! {
                "$match": {
                    "$expr": {
                        "$eq": [ "$_id", to_bson::<ObjectId>(_id).unwrap() ]
                    }
                }
            },
            doc! {
                "$lookup": {
                    "from": "customers",
                    "let": {
                        "customer_id": "$customer_id"
                    },
                    "as": "customers",
                    "pipeline": [
                        {
                            "$match": {
                                "$expr": {
                                    "$eq": ["$_id", "$$customer_id"]
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
                "$project": {
                    "_id": {
                        "$toString": "$_id"
                    },
                    "customer": {
                        "$first": "$customers"
                    },
                    "name": "$name",
                    "code": "$code",
                    "period": {
                        "start": { "$toString": "$period.start" },
                        "end": { "$toString": "$period.end" },
                    },
                    "status": "$status",
                    "area": "$area",
                    "leave": "$leave",
                }
            },
        ];

        match collection.aggregate(pipeline, None).await {
            Ok(mut cursor) => {
                if let Some(Ok(doc)) = cursor.next().await {
                    let user: ProjectResponse = from_document::<ProjectResponse>(doc).unwrap();
                    Ok(Some(user))
                } else {
                    Err("PROJECT_NOT_FOUND".to_string())
                }
            }
            Err(_) => Err("PROJECT_NOT_FOUND".to_string()),
        }
    }
    pub async fn find_users(_id: &ObjectId) -> Result<Option<ProjectUserResponse>, String> {
        let db: Database = get_db();
        let collection: Collection<Project> = db.collection::<Project>("projects");

        let pipeline: Vec<mongodb::bson::Document> = vec![
            doc! {
                "$match": {
                    "$expr": {
                        "$eq": ["$_id", to_bson::<ObjectId>(_id).unwrap()]
                    }
                }
            },
            doc! {
                "$lookup": {
                    "from": "users",
                    "let": {
                        "user": {
                            "$map": {
                                "input": {
                                    "$filter": {
                                        "input": "$member",
                                        "cond": {
                                            "$ne": ["$$this.kind", "support"]
                                        }
                                    }
                                },
                                "in": {
                                    "_id": "$$this._id",
                                    "role_id": "$$this.role_id",
                                    "kind": "$$this.kind"
                                }
                            }
                        },
                    },
                    "as": "user",
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
                                "_id": {
                                    "$toString": "$_id"
                                },
                                "role_id": {
                                    "$arrayElemAt": [
                                        "$$user.role_id",
                                        {
                                            "$indexOfArray": [
                                                "$$user._id",
                                                "$_id"
                                            ]
                                        }
                                    ]
                                },
                                "kind": {
                                    "$arrayElemAt": [
                                        "$$user.kind",
                                        {
                                            "$indexOfArray": [
                                                "$$user._id",
                                                "$_id"
                                            ]
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
                "$lookup": {
                    "from": "project-roles",
                    "let": {
                        "project_id": "$_id"
                    },
                    "as": "role",
                    "pipeline": [
                        {
                            "$match": {
                                "$expr": {
                                    "$eq": ["$project_id", "$$project_id"]
                                }
                            }
                        },
                        {
                            "$project": {
                                "_id": {
                                    "$toString": "$_id"
                                },
                                "name": "$name",
                                "permission": "$permission",
                            }
                        }
                    ]
                }
            },
            doc! {
                "$project": {
                    "user": {
                        "$concatArrays": [
                            "$user",
                            {
                                "$map": {
                                    "input": {
                                        "$filter": {
                                            "input": "$member",
                                            "cond": {
                                                "$eq": ["$$this.kind", "support"]
                                            }
                                        }
                                    },
                                    "in": {
                                        "_id": {
                                            "$toString": "$$this._id"
                                        },
                                        "name": "$$this.name",
                                        "kind": "$$this.kind",
                                        "role_id": {
                                            "$map": {
                                                "input": "$$this.role_id",
                                                "as": "role",
                                                "in": {
                                                    "$toString": "$$role"
                                                }
                                            }
                                        },
                                        "image": to_bson::<Option<UserImage>>(&None).unwrap()
                                    }
                                }
                            }
                        ]
                    },
                    "role": "$role"
                }
            },
            doc! {
                "$project": {
                    "user": {
                        "$map": {
                            "input": "$user",
                            "in": {
                                "_id": "$$this._id",
                                "name": "$$this.name",
                                "kind": "$$this.kind",
                                "image": "$$this.image",
                                "role": {
                                    "$map": {
                                        "input": "$$this.role_id",
                                        "in": {
                                            "_id": {
                                                "$toString": "$$this"
                                            },
                                            "name": {
                                                "$first": {
                                                    "$map": {
                                                        "input": {
                                                            "$filter": {
                                                                "input": "$role",
                                                                "as": "role",
                                                                "cond": {
                                                                    "$eq": ["$$role._id", { "$toString": "$$this" }]
                                                                }
                                                            }
                                                        },
                                                        "as": "role",
                                                        "in": "$$role.name"
                                                    }
                                                }
                                            },
                                            "permission": {
                                                "$first": {
                                                    "$map": {
                                                        "input": {
                                                            "$filter": {
                                                                "input": "$role",
                                                                "as": "role",
                                                                "cond": {
                                                                    "$eq": ["$$role._id", { "$toString": "$$this" }]
                                                                }
                                                            }
                                                        },
                                                        "as": "role",
                                                        "in": "$$role.permission"
                                                    }
                                                }
                                            },
                                        }
                                    }
                                }
                            }
                        }
                    },
                    "role": "$role"
                }
            },
        ];

        if let Ok(mut cursor) = collection.aggregate(pipeline, None).await {
            if let Some(Ok(doc)) = cursor.next().await {
                let user = from_document::<ProjectUserResponse>(doc).unwrap();
                Ok(Some(user))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }
    pub async fn delete_by_id(_id: &ObjectId) -> Result<u64, String> {
        let db: Database = get_db();
        let collection: Collection<Project> = db.collection::<Project>("projects");

        collection
            .delete_one(doc! { "_id": _id }, None)
            .await
            .map_err(|_| "PROJECT_NOT_FOUND".to_string())
            .map(|result| result.deleted_count)
    }
    pub async fn update_status(
        &mut self,
        status: ProjectStatusKind,
        message: Option<String>,
    ) -> Result<ObjectId, String> {
        let db: Database = get_db();
        let collection: Collection<Project> = db.collection::<Project>("projects");

        self.status.insert(
            0,
            ProjectStatus {
                kind: status.clone(),
                time: DateTime::from_millis(Utc::now().timestamp_millis()),
                message,
            },
        );

        if status == ProjectStatusKind::Running {
            let mut total: f64 = 0.0;
            let tasks = ProjectTask::find_many(&ProjectTaskQuery {
                _id: None,
                project_id: self._id,
                task_id: None,
                area_id: None,
                limit: None,
                kind: Some(ProjectTaskQueryKind::Root),
            })
            .await?
            .ok_or_else(|| "PROJECT_TASK_NOT_FOUND".to_string())?;

            for i in &tasks {
                total += i.value;
            }

            if total != 100.0 {
                return Err("PROJECT_TASK_VALUE_SUM_MUST_BE_100".to_string());
            }
        }

        collection
            .update_one(
                doc! { "_id": self._id.unwrap() },
                doc! { "$set": to_bson::<Self>(self).unwrap()},
                None,
            )
            .await
            .map_err(|_| "UPDATE_FAILED".to_string())
            .map(|_| self._id.unwrap())
    }
    pub async fn remove_area(&mut self, area_id: &ObjectId) -> Result<ObjectId, String> {
        let db: Database = get_db();
        let collection: Collection<Project> = db.collection::<Project>("projects");

        if let Some(area) = self.area.as_mut() {
            if let Some(index) = area.iter().position(|a| a._id == *area_id) {
                area.remove(index);
            }
        }

        collection
            .update_one(
                doc! { "_id": self._id.unwrap() },
                doc! { "$set": to_bson::<Project>(self).unwrap()},
                None,
            )
            .await
            .map_err(|_| "UPDATE_FAILED".to_string())
            .map(|_| self._id.unwrap())
    }
}
