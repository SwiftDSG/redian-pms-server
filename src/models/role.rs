use crate::database::get_db;
use futures::StreamExt;
use mongodb::{
    bson::{doc, from_document, oid::ObjectId, to_bson},
    Collection, Database,
};
use serde::{Deserialize, Serialize};

use super::user::User;

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RolePermission {
    Owner,
    GetUsers,
    GetUser,
    CreateUser,
    DeleteUser,
    UpdateUser,
    GetRoles,
    GetRole,
    CreateRole,
    DeleteRole,
    UpdateRole,
    GetCustomers,
    GetCustomer,
    CreateCustomer,
    DeleteCustomer,
    UpdateCustomer,
    GetProjects,
    GetProject,
    CreateProject,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Role {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _id: Option<ObjectId>,
    pub name: String,
    pub permission: Vec<RolePermission>,
}
#[derive(Debug)]
pub struct RoleQuery {
    pub _id: Option<ObjectId>,
    pub limit: Option<usize>,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct RoleRequest {
    pub name: String,
    pub permission: Vec<RolePermission>,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct RoleResponse {
    pub _id: String,
    pub name: String,
    pub permission: Vec<RolePermission>,
}

impl Role {
    pub async fn validate(ids: &[ObjectId], permit: &RolePermission) -> bool {
        for id in ids.iter() {
            if let Ok(Some(role)) = Self::find_by_id(id).await {
                if role.permission.iter().any(|permission| match permission {
                    RolePermission::Owner => true,
                    _ => permission == permit,
                }) {
                    return true;
                }
            }
        }
        false
    }
    pub fn set_as_owner(&mut self) {
        self.permission.push(RolePermission::Owner);
    }
    pub async fn save(&mut self) -> Result<ObjectId, String> {
        let db: Database = get_db();
        let collection: Collection<Role> = db.collection::<Role>("roles");

        self._id = Some(ObjectId::new());

        collection
            .insert_one(self, None)
            .await
            .map_err(|_| "INSERTING_FAILED".to_string())
            .map(|result| result.inserted_id.as_object_id().unwrap())
    }
    pub async fn update(&mut self) -> Result<ObjectId, String> {
        let db: Database = get_db();
        let collection: Collection<Role> = db.collection::<Role>("roles");

        collection
            .update_one(
                doc! { "_id": self._id.unwrap() },
                doc! { "$set": to_bson::<Self>(self).unwrap() },
                None,
            )
            .await
            .map_err(|_| "UPDATE_FAILED".to_string())
            .map(|_| self._id.unwrap())
    }
    pub async fn find_many(query: &RoleQuery) -> Result<Vec<RoleResponse>, String> {
        let db: Database = get_db();
        let collection: Collection<Role> = db.collection::<Role>("roles");

        let mut pipeline: Vec<mongodb::bson::Document> = Vec::new();
        let mut roles: Vec<RoleResponse> = Vec::new();

        if let Some(limit) = query.limit {
            pipeline.push(doc! {
                "$limit": to_bson::<usize>(&limit).unwrap()
            });
        }

        pipeline.push(doc! {
            "$project": {
                "_id": { "$toString": "$_id" },
                "name": "$name",
                "permission": "$permission",
            }
        });

        if let Ok(mut cursor) = collection.aggregate(pipeline, None).await {
            while let Some(Ok(doc)) = cursor.next().await {
                let role: RoleResponse = from_document::<RoleResponse>(doc).unwrap();
                roles.push(role)
            }
            if !roles.is_empty() {
                Ok(roles)
            } else {
                Err("ROLE_NOT_FOUND".to_string())
            }
        } else {
            Err("ROLE_NOT_FOUND".to_string())
        }
    }
    pub async fn find_by_id(_id: &ObjectId) -> Result<Option<Role>, String> {
        let db: Database = get_db();
        let collection: Collection<Role> = db.collection::<Role>("roles");

        collection
            .find_one(doc! { "_id": _id }, None)
            .await
            .map_err(|_| "ROLE_NOT_FOUND".to_string())
    }
    pub async fn delete_many() -> Result<u64, String> {
        let db: Database = get_db();
        let collection: Collection<Role> = db.collection::<Role>("roles");

        collection
            .delete_many(doc! {}, None)
            .await
            .map_err(|_| "ROLE_NOT_FOUND".to_string())
            .map(|result| result.deleted_count)
    }
    pub async fn delete_by_id(_id: &ObjectId) -> Result<u64, String> {
        let db: Database = get_db();
        let collection: Collection<Role> = db.collection::<Role>("roles");

        if let Ok(mut cursor) = db
            .collection::<User>("users")
            .find(
                doc! {
                    "role_id": to_bson::<ObjectId>(_id).unwrap()
                },
                None,
            )
            .await
        {
            while let Some(Ok(mut user)) = cursor.next().await {
                if let Some(index) = user.role_id.iter().position(|a| a == _id) {
                    user.role_id.remove(index);
                    if user.role_id.is_empty() {
                        user.delete()
                            .await
                            .map_err(|_| "USER_DELETION_FAILED".to_string())?;
                    } else {
                        user.update(false)
                            .await
                            .map_err(|_| "ROLE_DELETION_FAILED".to_string())?;
                    }
                }
            }
        }

        collection
            .delete_one(doc! { "_id": _id }, None)
            .await
            .map_err(|_| "ROLE_NOT_FOUND".to_string())
            .map(|result| result.deleted_count)
    }
}
