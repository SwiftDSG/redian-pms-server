use crate::database::get_db;
use mongodb::{
    bson::{doc, oid::ObjectId, to_bson, DateTime},
    Collection, Database,
};
use serde::{Deserialize, Serialize};

use super::{customer::Customer, user::User};

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum ProjectMemberKind {
    Direct,
    Indirect,
    Support,
}
#[derive(Clone, Serialize, Deserialize, Debug)]
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
#[derive(Debug, Deserialize, Serialize)]
pub struct ProjectPeriod {
    pub start: DateTime,
    pub end: DateTime,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ProjectMember {
    pub _id: ObjectId,
    pub name: Option<String>,
    pub kind: ProjectMemberKind,
    pub role_id: Vec<ObjectId>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ProjectStatus {
    pub kind: ProjectStatusKind,
    pub time: DateTime,
    pub message: Option<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ProjectArea {
    pub _id: ObjectId,
    pub name: String,
}
#[derive(Debug)]
pub struct ProjectQuery {
    pub _id: Option<ObjectId>,
    pub limit: Option<usize>,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct ProjectRequest {
    pub customer_id: ObjectId,
    pub name: String,
    pub code: String,
    pub status: Option<ProjectStatusKind>,
    pub holiday: Option<Vec<DateTime>>,
}

impl Project {
    pub async fn save(&self) -> Result<ObjectId, String> {
        let db: Database = get_db();
        let collection: Collection<Project> = db.collection::<Project>("projects");

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
    pub async fn add_member(&mut self, members: &Vec<ProjectMember>) -> Result<ObjectId, String> {
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
    pub async fn add_area(&mut self, areas: &Vec<ProjectArea>) -> Result<ObjectId, String> {
        let db: Database = get_db();
        let collection: Collection<Project> = db.collection::<Project>("projects");

        let mut area: Vec<ProjectArea> = match &self.area {
            Some(area) => Vec::<ProjectArea>::from_iter(area.clone()),
            None => Vec::<ProjectArea>::new(),
        };

        for i in areas.iter() {
            area.push(i.clone());
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
    pub async fn find_by_id(_id: &ObjectId) -> Result<Option<Project>, String> {
        let db: Database = get_db();
        let collection: Collection<Project> = db.collection::<Project>("projects");

        collection
            .find_one(doc! { "_id": _id }, None)
            .await
            .map_err(|_| "ROLE_NOT_FOUND".to_string())
    }
    // pub async fn add
}
