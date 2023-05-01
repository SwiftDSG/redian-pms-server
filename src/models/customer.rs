use crate::database::get_db;
use actix_service::{self, Transform};
use actix_web::{
    dev::{Service, ServiceRequest, ServiceResponse},
    Error, HttpMessage,
};
use chrono::Utc;
use futures::{
    future::{ready, LocalBoxFuture, Ready},
    stream::StreamExt,
    FutureExt,
};
use jsonwebtoken::{self, decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use mongodb::{
    bson::{doc, from_document, oid::ObjectId, to_bson},
    Collection, Database,
};
use pwhash::bcrypt;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs::read_to_string, rc::Rc, str::FromStr};

static mut KEYS: BTreeMap<String, String> = BTreeMap::new();

#[derive(Debug, Serialize, Deserialize)]
struct UserClaims {
    aud: String,
    exp: i64,
    iss: String,
    sub: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
pub struct Customer {
   #[serde(skip_serializing_if = "Option::is_none")]
  pub _id: Option<ObjectId>,
  pub name: String,
}
#[derive(Debug)]
pub struct CustomerContact {
  pub address: Option<String>,
  pub email: Option<String>,
  pub phone: Option<String>,
}
#[derive(Debug)]
pub struct CustomerPerson {
  pub name: String,
  pub address: Option<String>,
  pub phone: Option<String>,
  pub email: Option<String>,
  pub role: String,
}

impl Customer { 
  pub async fn save(&mut self) -> Result<ObjectId, String> { 
    let db: Database = get_db();
    let colletion: Collection<Customer> = db.collection::<Customer>("customers");

    self._id = Some(ObjectId::new());

    Err("error".to_string())
  } 
  // pub async fn
}
