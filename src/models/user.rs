use futures::stream::TryStreamExt;
use mongodb::{
    bson::{doc, oid::ObjectId, ser, to_bson, Document},
    error, Collection, Cursor, Database,
};
use serde::{Deserialize, Serialize};

use crate::database::get_db;

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
pub struct User {
    pub _id: Option<ObjectId>,
    pub name: String,
    pub email: String,
    pub password: String,
    pub role: String,
}

impl User {
    pub fn as_document(&self) -> Result<Document, ser::Error> {
        let serialized = to_bson(&self)?;
        let document = serialized.as_document().unwrap();
        Ok(document.clone())
    }
}

impl User {
    pub fn new() -> Self {
        Self {
            _id: Some(ObjectId::new()),
            name: String::new(),
            email: String::new(),
            password: String::new(),
            role: String::new(),
        }
    }
    pub async fn find_many() -> Result<Vec<User>, error::Error> {
        let db: Database = get_db();

        let collection: Collection<User> = db.collection::<User>("users");
        let cursor: Cursor<User> = collection.find(doc! {}, None).await?;

        cursor.try_collect().await
    }
    pub async fn find_one(_id: ObjectId) -> Result<Option<User>, error::Error> {
        let db: Database = get_db();

        let collection: Collection<User> = db.collection::<User>("users");

        collection.find_one(doc! { "_id": _id }, None).await
    }
}
