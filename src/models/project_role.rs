use crate::database::get_db;

use mongodb::{
    bson::{doc, oid::ObjectId, to_bson},
    Collection, Database,
};
use serde::{Deserialize, Serialize};

use super::project::Project;

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProjectRolePermission {
    Owner,
    CreateRole,
    UpdateRole,
    DeleteRole,
    GetRoles,
    GetRole,
    CreateTask,
    UpdateTask,
    DeleteTask,
    GetTasks,
    GetTask,
    CreateReport,
    CreateIncident,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ProjectRole {
    pub _id: Option<ObjectId>,
    pub project_id: ObjectId,
    pub name: String,
    pub permission: Vec<ProjectRolePermission>,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct ProjectRoleRequest {
    pub name: String,
    pub permission: Vec<ProjectRolePermission>,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct ProjectRoleResponse {
    pub _id: String,
    pub name: String,
    pub permission: Vec<ProjectRolePermission>,
}
#[derive(Debug)]
pub struct ProjectRoleQuery {
    pub project_id: Option<ObjectId>,
}

impl ProjectRole {
    pub async fn validate(
        project_id: &ObjectId,
        user_id: &ObjectId,
        permit: &ProjectRolePermission,
    ) -> bool {
        if let Ok(Some(project)) = Project::find_by_id(project_id).await {
            if let Some(members) = &project.member {
                if let Some(member) = members.iter().find(|&a| a._id == *user_id) {
                    for id in &member.role_id {
                        if let Ok(Some(role)) = Self::find_by_id(id).await {
                            if role.permission.iter().any(|permission| match permission {
                                ProjectRolePermission::Owner => true,
                                _ => permission == permit,
                            }) {
                                return true;
                            }
                        }
                    }
                }
            }
        }
        false
    }
    pub async fn save(&mut self) -> Result<ObjectId, String> {
        let db: Database = get_db();
        let collection: Collection<ProjectRole> = db.collection::<ProjectRole>("project-roles");

        self._id = Some(ObjectId::new());

        if let Ok(Some(_)) = Project::find_by_id(&self.project_id).await {
            collection
                .insert_one(self, None)
                .await
                .map_err(|_| "INSERTING_FAILED".to_string())
                .map(|result| result.inserted_id.as_object_id().unwrap())
        } else {
            Err("PROJECT_NOT_FOUND".to_string())
        }
    }
    pub async fn find_by_id(_id: &ObjectId) -> Result<Option<ProjectRole>, String> {
        let db: Database = get_db();
        let collection: Collection<ProjectRole> = db.collection::<ProjectRole>("project-roles");

        collection
            .find_one(doc! { "_id": _id }, None)
            .await
            .map_err(|_| "PROJECT_ROLE_NOT_FOUND".to_string())
    }
    pub async fn delete_by_id(_id: &ObjectId) -> Result<u64, String> {
        let db: Database = get_db();
        let collection: Collection<ProjectRole> = db.collection::<ProjectRole>("project-roles");

        collection
            .delete_one(doc! { "_id": _id }, None)
            .await
            .map_err(|_| "PROJECT_ROLE_NOT_FOUND".to_string())
            .map(|result| result.deleted_count)
    }
    pub async fn update(&self) -> Result<ObjectId, String> {
        let db: Database = get_db();
        let collection: Collection<ProjectRole> = db.collection::<ProjectRole>("project-roles");

        collection
            .update_one(
                doc! {
                    "_id": self._id.unwrap()
                },
                doc! {
                    "$set": to_bson::<Self>(self).unwrap()
                },
                None,
            )
            .await
            .map_err(|_| "UPDATE_FAILED".to_string())
            .map(|_| self._id.unwrap())
    }
}
