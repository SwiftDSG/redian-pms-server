use crate::database::get_db;
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
    pub contact: CustomerContact,
    pub person: Vec<CustomerPerson>,
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
#[derive(Debug)]
pub struct CustomerQuery {
    pub _id: Option<ObjectId>,
    pub name: Option<String>,
    pub limit: Option<usize>,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct CustomerRequest {
    pub name: String,
    pub contact: CustomerContact,
    pub person: Vec<CustomerPerson>,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct CustomerResponse {
    pub _id: Option<ObjectId>,
    pub name: String,
    pub contact: CustomerContact,
    pub person: Vec<CustomerPerson>,
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

        // Err("error".to_string());
    }
    pub async fn find_many(query: &CustomerQuery) -> Result<Vec<CustomerResponse>, String> {
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
            "name" : "$name",
            "contact" : "$contact",
            "person" : "$person",
          }
        });

        if let Ok(mut cursor) = collection.aggregate(pipeline, None).await {
            while let Some(Ok(doc)) = cursor.next().await {
                let customer: CustomerResponse = from_document::<CustomerResponse>(doc).unwrap();
                customers.push(customer);
            }
            if !customers.is_empty() {
                Ok(customers)
            } else {
                Err("error".to_string())
            }
        } else {
            Err("error".to_string())
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
