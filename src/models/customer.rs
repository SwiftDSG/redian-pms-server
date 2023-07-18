use crate::database::get_db;
use actix_multipart::form::{tempfile::TempFile, MultipartForm};
use futures::stream::StreamExt;
use mongodb::{
    bson::{doc, from_document, oid::ObjectId, to_bson},
    Collection, Database,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct Customer {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _id: Option<ObjectId>,
    pub name: String,
    pub field: String,
    pub contact: CustomerContact,
    pub person: Vec<CustomerPerson>,
    pub image: Option<CustomerImage>,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct CustomerContact {
    pub address: String,
    pub email: Option<String>,
    pub phone: Option<String>,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct CustomerPerson {
    pub _id: Option<ObjectId>,
    pub name: String,
    pub address: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub role: String,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct CustomerImage {
    pub _id: ObjectId,
    pub extension: String,
}
#[derive(Debug)]
pub struct CustomerQuery {
    pub _id: Option<ObjectId>,
    pub name: Option<String>,
    pub limit: Option<usize>,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct CustomerRequest {
    pub name: String,
    pub field: String,
    pub contact: CustomerContact,
    pub person: Vec<CustomerPerson>,
    pub image: Option<CustomerImageRequest>,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct CustomerImageRequest {
    pub extension: String,
}
#[derive(Debug, MultipartForm)]
pub struct CustomerImageMultipartRequest {
    #[multipart(rename = "file")]
    pub file: TempFile,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct CustomerResponse {
    pub _id: String,
    pub name: String,
    pub field: String,
    pub contact: CustomerContact,
    pub person: Vec<CustomerPersonResponse>,
    pub image: Option<CustomerImageResponse>,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct CustomerPersonResponse {
    pub _id: String,
    pub name: String,
    pub address: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub role: String,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct CustomerImageResponse {
    pub _id: String,
    pub extension: String,
}

impl Customer {
    pub async fn save(&mut self) -> Result<ObjectId, String> {
        let db: Database = get_db();
        let collection: Collection<Customer> = db.collection::<Customer>("customers");

        self._id = Some(ObjectId::new());

        for person in self.person.iter_mut() {
            person._id = Some(ObjectId::new());
        }

        collection
            .insert_one(self, None)
            .await
            .map_err(|_| "INSERTING_FAILED".to_string())
            .map(|result| result.inserted_id.as_object_id().unwrap())
    }
    pub async fn update(&self) -> Result<ObjectId, String> {
        let db: Database = get_db();
        let collection: Collection<Customer> = db.collection::<Customer>("customers");

        collection
            .update_one(
                doc! { "_id": self._id.unwrap() },
                doc! { "$set": to_bson::<Customer>(self).unwrap()},
                None,
            )
            .await
            .map_err(|_| "UPDATE_FAILED".to_string())
            .map(|_| self._id.unwrap())
    }
    pub async fn delete(&self) -> Result<u64, String> {
        let db: Database = get_db();
        let collection: Collection<Customer> = db.collection::<Customer>("customers");

        collection
            .delete_one(doc! { "_id": self._id.unwrap() }, None)
            .await
            .map_err(|_| "CUSTOMER_NOT_FOUND".to_string())
            .map(|result| result.deleted_count)
    }
    pub async fn find_many(query: &CustomerQuery) -> Result<Option<Vec<CustomerResponse>>, String> {
        let db: Database = get_db();
        let collection: Collection<Customer> = db.collection::<Customer>("customers");

        let mut pipeline: Vec<mongodb::bson::Document> = Vec::new();
        let mut customers: Vec<CustomerResponse> = Vec::new();

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
            "name" : "$name",
            "field" : "$field",
            "contact" : "$contact",
            "person" : {
                "$map": {
                    "input": "$person",
                    "in": {
                        "_id": {
                            "$toString": "$$this._id"
                        },
                        "name" : "$$this.name",
                        "address" : "$$this.address",
                        "phone" : "$$this.phone",
                        "email" : "$$this.email",
                        "role" : "$$this.role",
                    }
                }
            },
            "image": {
                "$cond": [
                    "$image",
                    {
                        "_id": {
                            "$toString": "$image._id"
                        },
                        "extension": "$image.extension"
                    },
                    to_bson::<Option<CustomerImageResponse>>(&None).unwrap()
                ]
            },
          }
        });

        if let Ok(mut cursor) = collection.aggregate(pipeline, None).await {
            while let Some(Ok(doc)) = cursor.next().await {
                let customer: CustomerResponse = from_document::<CustomerResponse>(doc).unwrap();
                customers.push(customer);
            }
            if !customers.is_empty() {
                Ok(Some(customers))
            } else {
                Ok(None)
            }
        } else {
            Err("CUSTOMER_NOT_FOUND".to_string())
        }
    }
    pub async fn find_by_id(_id: &ObjectId) -> Result<Option<Customer>, String> {
        let db: Database = get_db();
        let collection: Collection<Customer> = db.collection::<Customer>("customers");

        collection
            .find_one(doc! { "_id": _id }, None)
            .await
            .map_err(|_| "CUSTOMER_NOT_FOUND".to_string())
    }
}
