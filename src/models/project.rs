use crate::database::get_db;
use mongodb::{
    bson::{doc, from_document, oid::ObjectId, to_bson, DateTime},
    Collection, Database,
};
use serde::{Deserialize, Serialize};

use super::user::User;

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum ProjectMemberKind {
    Direct,
    Indirect,
    Support,
}
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum ProjectStatus {
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
    pub status: ProjectStatus,
    pub member: Option<Vec<ProjectMember>>,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct ProjectPeriodPlan {
    pub start: DateTime,
    pub end: DateTime,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct ProjectPeriodActual {
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
    pub status: Option<ProjectStatus>,
}

impl Project {
    pub async fn save(&mut self) -> Result<ObjectId, String> {
        let db: Database = get_db();
        let collection: Collection<Project> = db.collection::<Project>("projects");

        self._id = Some(ObjectId::new());

        collection
            .insert_one(self, None)
            .await
            .map_err(|_| "INSERTING_FAILED".to_string())
            .map(|result| result.inserted_id.as_object_id().unwrap())
    }
    pub async fn add_member(&mut self, members: &Vec<ProjectMember>) -> Result<ObjectId, String> {
        let db: Database = get_db();
        let collection: Collection<Project> = db.collection::<Project>("projects");

        let mut member = match &self.member {
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
    // pub async fn add
}
