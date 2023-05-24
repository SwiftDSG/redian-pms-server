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
    project_task::{ProjectTask, ProjectTaskQuery},
    user::User,
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
    pub status: Vec<ProjectStatus>,
    pub area: Option<Vec<ProjectArea>>,
    pub member: Option<Vec<ProjectMember>>,
    pub holiday: Option<Vec<DateTime>>,
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
    pub status: Option<ProjectStatusKind>,
    pub holiday: Option<Vec<DateTime>>,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct ProjectResponse {
    pub _id: Option<String>,
    pub customer_id: String,
    pub name: String,
    pub code: String,
    pub status: Vec<ProjectStatus>,
    pub area: Option<Vec<ProjectArea>>,
    pub member: Option<Vec<ProjectMember>>,
    pub holiday: Option<Vec<DateTime>>,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct ProjectDetailResponse {
    pub _id: Option<String>,
    pub customer_id: String,
    pub name: String,
    pub code: String,
    pub status: Vec<ProjectStatus>,
    pub area: Option<Vec<ProjectArea>>,
    pub member: Option<Vec<ProjectMember>>,
    pub holiday: Option<Vec<DateTime>>,
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
    pub async fn add_member(&mut self, members: &[ProjectMember]) -> Result<ObjectId, String> {
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
                        member.push(i.clone());
                    }
                }
                _ => {
                    if (User::find_by_id(&i._id).await).is_ok() {
                        member.push(i.clone());
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
    pub async fn find_many(query: &ProjectQuery) -> Result<Vec<ProjectResponse>, String> {
        let db: Database = get_db();
        let collection: Collection<Project> = db.collection::<Project>("projects");

        let mut pipeline: Vec<mongodb::bson::Document> = Vec::new();
        let mut users: Vec<ProjectResponse> = Vec::new();

        if let Some(limit) = query.limit {
            pipeline.push(doc! {
                "$limit": to_bson::<usize>(&limit).unwrap()
            })
        }

        pipeline.push(doc! {
            "$project": {
                "_id": {
                    "$toString": "$_id"
                },
                "customer_id": {
                    "$toString": "$customer_id"
                },
                "name": "$name",
                "code": "$code",
                "status": "$status",
                "area": "$area",
                "member": "$member",
                "holiday": "$holiday",
            }
        });

        if let Ok(mut cursor) = collection.aggregate(pipeline, None).await {
            while let Some(Ok(doc)) = cursor.next().await {
                let user: ProjectResponse = from_document::<ProjectResponse>(doc).unwrap();
                users.push(user);
            }
            if !users.is_empty() {
                Ok(users)
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
    pub async fn find_detail_by_id(
        _id: &ObjectId,
    ) -> Result<Option<ProjectDetailResponse>, String> {
        let db: Database = get_db();
        let collection: Collection<Project> = db.collection::<Project>("projects");

        let mut pipeline: Vec<mongodb::bson::Document> = Vec::new();

        pipeline.push(doc! {
            "$project": {
                "_id": {
                    "$toString": "$_id"
                },
                "customer_id": {
                    "$toString": "$customer_id"
                },
                "name": "$name",
                "code": "$code",
                "status": "$status",
                "area": "$area",
                "member": "$member",
                "holiday": "$holiday",
            }
        });

        if let Ok(mut cursor) = collection.aggregate(pipeline, None).await {
            if let Some(Ok(doc)) = cursor.next().await {
                let user: ProjectDetailResponse =
                    from_document::<ProjectDetailResponse>(doc).unwrap();
                Ok(Some(user))
            } else {
                Err("PROJECT_NOT_FOUND".to_string())
            }
        } else {
            Err("PROJECT_NOT_FOUND".to_string())
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
                base: true,
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
                doc! { "$set": to_bson::<Project>(self).unwrap()},
                None,
            )
            .await
            .map_err(|_| "UPDATE_FAILED".to_string())
            .map(|_| self._id.unwrap())
    }
    // pub async fn add
}
