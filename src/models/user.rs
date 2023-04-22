use crate::database::get_db;
use chrono::Utc;
use futures::stream::TryStreamExt;
use jsonwebtoken::{self, decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use mongodb::{
    bson::{doc, oid::ObjectId},
    Collection, Database,
};
use pwhash::bcrypt;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs::read_to_string};

static mut KEYS: BTreeMap<String, String> = BTreeMap::new();

#[derive(Debug, Serialize, Deserialize)]
struct UserClaims {
    aud: String,
    exp: i64,
    iss: String,
    sub: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
pub struct User {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _id: Option<ObjectId>,
    pub name: String,
    pub email: String,
    pub password: String,
    pub role: String,
}
#[derive(Clone, Debug, Deserialize, Serialize, Default)]
pub struct UserCredential {
    pub email: String,
    pub password: String,
}

impl User {
    pub async fn save(&self) -> Result<ObjectId, String> {
        let db: Database = get_db();
        let collection: Collection<User> = db.collection::<User>("users");

        let mut user: User = Self {
            _id: Some(ObjectId::new()),
            ..self.clone()
        };

        if let Ok(hash) = bcrypt::hash(&user.password) {
            user.password = hash;
            collection
                .insert_one(user, None)
                .await
                .map_err(|_| String::from("INSERTING_FAILED"))
                .map(|result| result)
                .and_then(|result| Ok(result.inserted_id.as_object_id().unwrap()))
        } else {
            Err(String::from("HASHING_FAILED"))
        }
    }
    pub async fn find_many() -> Result<Vec<User>, String> {
        let db: Database = get_db();
        let collection: Collection<User> = db.collection::<User>("users");

        if let Ok(cursor) = collection.find(doc! {}, None).await {
            return cursor
                .try_collect()
                .await
                .map_err(|_| String::from("COLLECTING_FAILED"));
        } else {
            Err(String::from("USER_NOT_FOUND"))
        }
    }
    pub async fn find_by_id(_id: &ObjectId) -> Result<Option<User>, String> {
        let db: Database = get_db();
        let collection: Collection<User> = db.collection::<User>("users");

        collection
            .find_one(doc! { "_id": _id }, None)
            .await
            .map_err(|_| String::from("USER_NOT_FOUND"))
    }
    pub async fn find_by_email(email: &String) -> Result<Option<User>, String> {
        let db: Database = get_db();
        let collection: Collection<User> = db.collection::<User>("users");

        collection
            .find_one(doc! { "email": email }, None)
            .await
            .map_err(|_| String::from("USER_NOT_FOUND"))
    }
}

impl UserCredential {
    pub async fn authenticate(&self) -> Result<String, String> {
        if let Ok(Some(user)) = User::find_by_email(&self.email).await {
            if bcrypt::verify(self.password.clone(), &user.password) {
                let claims: UserClaims = UserClaims {
                    sub: ObjectId::to_string(&user._id.unwrap()),
                    exp: Utc::now().timestamp_millis() + 86400000,
                    iss: String::from("Redian"),
                    aud: String::from("http://localhost:8000"),
                };

                let header: Header = Header::new(Algorithm::RS256);
                unsafe {
                    if let Ok(token) = encode(
                        &header,
                        &claims,
                        &EncodingKey::from_rsa_pem(&KEYS.get("private_access").unwrap().as_bytes())
                            .unwrap(),
                    ) {
                        return Ok(token);
                    } else {
                        Err(String::from("GENERATING_FAILED"))
                    }
                }
            } else {
                Err(String::from("INVALID_COMBINATION"))
            }
        } else {
            Err(String::from("INVALID_COMBINATION"))
        }
    }
    pub fn verify(token: &String) -> bool {
        let validation: Validation = Validation::new(Algorithm::RS256);
        unsafe {
            if let Ok(_) = decode::<UserClaims>(
                token,
                &DecodingKey::from_rsa_pem(&KEYS.get("public_access").unwrap().as_bytes()).unwrap(),
                &validation,
            ) {
                return true;
            } else {
                false
            }
        }
    }
}

pub fn load_keys() {
    let private_access_file =
        read_to_string("./keys/private_access.key").expect("LOAD_FAILED_PRIVATE_ACCESS");
    let public_access_file =
        read_to_string("./keys/public_access.pem").expect("LOAD_FAILED_PUBLIC_ACCESS");
    unsafe {
        KEYS.insert(String::from("private_access"), private_access_file);
        KEYS.insert(String::from("public_access"), public_access_file);
    }
}
