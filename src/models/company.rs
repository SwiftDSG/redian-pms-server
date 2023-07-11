use crate::database::get_db;
use actix_multipart::form::{tempfile::TempFile, MultipartForm};
use futures::stream::StreamExt;
use mongodb::{
    bson::{doc, from_document, oid::ObjectId, to_bson},
    Collection, Database,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct Company {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _id: Option<ObjectId>,
    pub name: String,
    pub field: String,
    pub contact: CompanyContact,
    pub image: Option<CompanyImage>,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct CompanyContact {
    pub address: String,
    pub email: Option<String>,
    pub phone: Option<String>,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct CompanyImage {
    pub _id: ObjectId,
    pub extension: String,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct CompanyRequest {
    pub name: String,
    pub field: String,
    pub contact: CompanyContact,
    pub image: Option<CompanyImageRequest>,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct CompanyImageRequest {
    pub extension: String,
}
#[derive(Debug, MultipartForm)]
pub struct CompanyImageMultipartRequest {
    #[multipart(rename = "file")]
    pub file: TempFile,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct CompanyResponse {
    pub _id: String,
    pub name: String,
    pub field: String,
    pub contact: CompanyContactResponse,
    pub image: Option<CompanyImageResponse>,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct CompanyContactResponse {
    pub address: String,
    pub email: Option<String>,
    pub phone: Option<String>,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct CompanyImageResponse {
    pub _id: String,
    pub extension: String,
}

impl Company {
    pub async fn save(&mut self) -> Result<ObjectId, String> {
        let db: Database = get_db();
        let collection: Collection<Company> = db.collection::<Company>("companies");

        self._id = Some(ObjectId::new());

        collection
            .insert_one(self, None)
            .await
            .map_err(|_| "INSERTING_FAILED".to_string())
            .map(|result| result.inserted_id.as_object_id().unwrap())
    }
    pub async fn update(&self) -> Result<ObjectId, String> {
        let db: Database = get_db();
        let collection: Collection<Company> = db.collection::<Company>("companies");

        collection
            .update_one(
                doc! { "_id": self._id.unwrap() },
                doc! { "$set": to_bson::<Company>(self).unwrap()},
                None,
            )
            .await
            .map_err(|_| "UPDATE_FAILED".to_string())
            .map(|_| self._id.unwrap())
    }
    pub async fn find_by_id(_id: &ObjectId) -> Result<Option<Company>, String> {
        let db: Database = get_db();
        let collection: Collection<Company> = db.collection::<Company>("companies");

        collection
            .find_one(doc! { "_id": _id }, None)
            .await
            .map_err(|_| "COMPANY_NOT_FOUND".to_string())
    }
    pub async fn find_detail() -> Result<Option<CompanyResponse>, String> {
        let db: Database = get_db();
        let collection: Collection<Company> = db.collection::<Company>("companies");

        let pipeline = vec![doc! {
          "$project": {
            "_id": {
              "$toString": "$_id"
            },
            "name": "$name",
            "field": "$field",
            "contact": "$contact",
            "image": {
              "$cond": [
                "$image",
                {
                  "_id": {
                    "$toString": "$image._id"
                  },
                  "extension": "$image.extension"
                },
                to_bson::<Option<CompanyImageResponse>>(&None).unwrap()
              ]
            },
          }
        }];

        if let Ok(mut cursor) = collection.aggregate(pipeline, None).await {
            if let Some(Ok(doc)) = cursor.next().await {
                let company = from_document::<CompanyResponse>(doc).unwrap();
                Ok(Some(company))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }
}
