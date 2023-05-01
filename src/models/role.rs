use crate::database::get_db;
use futures::StreamExt;
use mongodb::{
    bson::{doc, from_document, oid::ObjectId, to_bson},
    Collection, Database,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Role {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _id: Option<ObjectId>,
    pub name: String,
    pub permission: Vec<String>,
}
#[derive(Debug)]
pub struct RoleQuery {
    pub _id: Option<ObjectId>,
    pub limit: Option<usize>,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct RoleRequest {
    pub name: String,
    pub permission: Vec<String>,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct RoleResponse {
    pub _id: Option<ObjectId>,
    pub name: String,
    pub permission: Vec<String>,
}

impl Role {
    pub fn add_permission(&mut self, payload: &str) {
        match payload {
            "get_users" => self.permission.push(payload.to_string()),
            "get_user" => self.permission.push(payload.to_string()),
            "add_user" => self.permission.push(payload.to_string()),
            "get_roles" => self.permission.push(payload.to_string()),
            "get_role" => self.permission.push(payload.to_string()),
            "add_role" => self.permission.push(payload.to_string()),
            "add_customer" => self.permission.push(payload.to_string()),
            "get_customer" => self.permission.push(payload.to_string()),
            "update_customer" => self.permission.push(payload.to_string()),
            _ => (),
        };
    }
    pub async fn validate(ids: &[ObjectId], action: &String) -> bool {
        for id in ids.iter() {
            if let Ok(Some(role)) = Self::find_by_id(id).await {
                if role.permission.contains(&"Owner".to_string())
                    || role.permission.contains(action)
                {
                    return true;
                }
            }
        }
        false
    }
    pub fn set_as_owner(&mut self) {
        self.permission.push("Owner".to_string());
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
    pub async fn find_many(query: &RoleQuery) -> Result<Vec<RoleResponse>, String> {
        let db: Database = get_db();
        let collection: Collection<Role> = db.collection::<Role>("roles");

        let mut pipeline: Vec<mongodb::bson::Document> = Vec::new();
        let mut roles: Vec<RoleResponse> = Vec::new();

        if let Some(limit) = query.limit {
            pipeline.push(doc! {
                "$limit": to_bson::<usize>(&limit).unwrap()
            })
        }

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

        collection
            .delete_one(doc! { "_id": _id }, None)
            .await
            .map_err(|_| "ROLE_NOT_FOUND".to_string())
            .map(|result| result.deleted_count)
    }
}
