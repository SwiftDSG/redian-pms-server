use crate::database::get_db;
use actix_multipart::form::{tempfile::TempFile, MultipartForm};
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
use jsonwebtoken::{
    self, decode, encode, Algorithm, DecodingKey, EncodingKey, Header, TokenData, Validation,
};
use mongodb::{
    bson::{doc, from_document, oid::ObjectId, to_bson},
    Collection, Database,
};
use pwhash::bcrypt;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs::read_to_string, rc::Rc, str::FromStr};

use super::role::RoleResponse;

static mut KEYS: BTreeMap<String, String> = BTreeMap::new();

#[derive(Debug, Deserialize, Serialize)]
pub struct User {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _id: Option<ObjectId>,
    pub role_id: Vec<ObjectId>,
    pub name: String,
    pub email: String,
    pub password: String,
    pub image: Option<UserImage>,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct UserImage {
    pub _id: ObjectId,
    pub extension: String,
}
#[derive(Debug, Deserialize)]
pub struct UserCredential {
    pub email: String,
    pub password: String,
}
#[derive(Debug, Deserialize)]
pub struct UserRefreshRequest {
    pub rtk: String,
}
#[derive(Debug)]
pub struct UserQuery {
    pub _id: Option<ObjectId>,
    pub role_id: Option<ObjectId>,
    pub email: Option<String>,
    pub limit: Option<usize>,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct UserRequest {
    pub role_id: Option<Vec<ObjectId>>,
    pub name: String,
    pub email: String,
    pub password: String,
    pub image: Option<UserImageRequest>,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct UserImageRequest {
    pub extension: String,
}
#[derive(Debug, MultipartForm)]
pub struct UserImageMultipartRequest {
    #[multipart(rename = "file")]
    pub file: TempFile,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UserResponse {
    pub _id: String,
    pub role: Vec<RoleResponse>,
    pub name: String,
    pub email: String,
    pub image: Option<UserImageResponse>,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct UserImageResponse {
    pub _id: String,
    pub extension: String,
}

#[derive(Debug)]
pub struct UserAuthenticationData {
    pub _id: Option<ObjectId>,
    pub role_id: Vec<ObjectId>,
    pub token: String,
}
#[derive(Debug, Serialize, Deserialize)]
struct UserClaim {
    aud: String,
    exp: i64,
    iss: String,
    sub: String,
}
pub struct UserAuthenticationMiddleware<S> {
    service: Rc<S>,
}
pub struct UserAuthenticationMiddlewareFactory;

pub type UserAuthentication = Rc<UserAuthenticationData>;

impl User {
    pub async fn save(&mut self) -> Result<ObjectId, String> {
        let db: Database = get_db();
        let collection: Collection<User> = db.collection::<User>("users");

        self._id = Some(ObjectId::new());

        if let Ok(hash) = bcrypt::hash(&self.password) {
            self.password = hash;
            collection
                .insert_one(self, None)
                .await
                .map_err(|_| "INSERTING_FAILED".to_string())
                .map(|result| result.inserted_id.as_object_id().unwrap())
        } else {
            Err("HASHING_FAILED".to_string())
        }
    }
    pub async fn update(&mut self, update_hash: bool) -> Result<ObjectId, String> {
        let db: Database = get_db();
        let collection: Collection<User> = db.collection::<User>("users");

        if update_hash {
            if let Ok(hash) = bcrypt::hash(&self.password) {
                self.password = hash;
            } else {
                return Err("HASHING_FAILED".to_string());
            }
        }

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
    pub async fn delete(&self) -> Result<u64, String> {
        let db: Database = get_db();
        let collection: Collection<User> = db.collection::<User>("users");

        collection
            .delete_one(doc! { "_id": self._id.unwrap() }, None)
            .await
            .map_err(|_| "USER_NOT_FOUND".to_string())
            .map(|result| result.deleted_count)
    }
    pub async fn find_many(query: &UserQuery) -> Result<Vec<UserResponse>, String> {
        let db: Database = get_db();
        let collection: Collection<User> = db.collection::<User>("users");

        let mut pipeline: Vec<mongodb::bson::Document> = Vec::new();
        let mut users: Vec<UserResponse> = Vec::new();

        if let Some(_id) = query.role_id {
            pipeline.push(doc! {
                "$match": {
                    "$expr": {
                        "$in": [to_bson::<ObjectId>(&_id).unwrap(), "$role_id"]
                    }
                }
            })
        }
        if let Some(limit) = query.limit {
            pipeline.push(doc! {
                "$limit": to_bson::<usize>(&limit).unwrap()
            })
        }

        pipeline.push(doc! {
            "$lookup": {
                "from": "roles",
                "as": "role",
                "let": {
                    "role_id": "$role_id"
                },
                "pipeline": [
                    {
                        "$match": {
                            "$expr": {
                                "$in": ["$_id", "$$role_id"]
                            }
                        }
                    },
                    {
                        "$project": {
                            "_id": { "$toString": "$_id" },
                            "name": "$name",
                            "permission": "$permission",
                        }
                    }
                ]
            }
        });
        pipeline.push(doc! {
            "$project": {
                "_id": {
                    "$toString": "$_id"
                },
                "name": "$name",
                "email": "$email",
                "role": "$role",
                "image": {
                    "$cond": [
                        "$image",
                        {
                            "_id": {
                                "$toString": "$image._id"
                            },
                            "extension": "$image.extension"
                        },
                        to_bson::<Option<UserImageResponse>>(&None).unwrap()
                    ]
                },
            }
        });

        if let Ok(mut cursor) = collection.aggregate(pipeline, None).await {
            while let Some(Ok(doc)) = cursor.next().await {
                let user = from_document::<UserResponse>(doc).unwrap();
                users.push(user);
            }
            if !users.is_empty() {
                Ok(users)
            } else {
                Err("USER_NOT_FOUND".to_string())
            }
        } else {
            Err("USER_NOT_FOUND".to_string())
        }
    }
    pub async fn find_by_id(_id: &ObjectId) -> Result<Option<User>, String> {
        let db: Database = get_db();
        let collection: Collection<User> = db.collection::<User>("users");

        collection
            .find_one(doc! { "_id": _id }, None)
            .await
            .map_err(|_| "USER_NOT_FOUND".to_string())
    }
    pub async fn find_by_email(email: &String) -> Result<Option<User>, String> {
        let db: Database = get_db();
        let collection: Collection<User> = db.collection::<User>("users");

        collection
            .find_one(doc! { "email": email }, None)
            .await
            .map_err(|_| "USER_NOT_FOUND".to_string())
    }
    pub async fn find_detail_by_id(_id: &ObjectId) -> Result<Option<UserResponse>, String> {
        let db: Database = get_db();
        let collection: Collection<User> = db.collection::<User>("users");

        let mut pipeline: Vec<mongodb::bson::Document> = Vec::new();

        pipeline.push(doc! {
            "$match": {
                "$expr": {
                    "$eq": ["$_id", to_bson::<ObjectId>(_id).unwrap()]
                }
            }
        });
        pipeline.push(doc! {
            "$lookup": {
                "from": "roles",
                "as": "role",
                "let": {
                    "role_id": "$role_id"
                },
                "pipeline": [
                    {
                        "$match": {
                            "$expr": {
                                "$in": ["$_id", "$$role_id"]
                            }
                        }
                    },
                    {
                        "$project": {
                            "_id": { "$toString": "$_id" },
                            "name": "$name",
                            "permission": "$permission",
                        }
                    }
                ]
            }
        });
        pipeline.push(doc! {
            "$project": {
                "_id": {
                    "$toString": "$_id"
                },
                "name": "$name",
                "email": "$email",
                "role": "$role",
                "image": {
                    "$cond": [
                        "$image",
                        {
                            "_id": {
                                "$toString": "$image._id"
                            },
                            "extension": "$image.extension"
                        },
                        to_bson::<Option<UserImageResponse>>(&None).unwrap()
                    ]
                },
            }
        });

        if let Ok(mut cursor) = collection.aggregate(pipeline, None).await {
            if let Some(Ok(doc)) = cursor.next().await {
                let user = from_document::<UserResponse>(doc).unwrap();
                Ok(Some(user))
            } else {
                Err("USER_NOT_FOUND".to_string())
            }
        } else {
            Err("USER_NOT_FOUND".to_string())
        }
    }
}

impl UserCredential {
    pub async fn authenticate(&self) -> Result<(String, String, UserResponse), String> {
        let user = User::find_by_email(&self.email)
            .await?
            .ok_or_else(|| "INVALID_COMBINATION".to_string())?;
        if !bcrypt::verify(self.password.clone(), &user.password) {
            return Err("INVALID_COMBINATION".to_string());
        }

        let claim_access: UserClaim = UserClaim {
            sub: ObjectId::to_string(&user._id.unwrap()),
            exp: Utc::now().timestamp() + 1800,
            iss: "Redian".to_string(),
            aud: std::env::var("BASE_URL").unwrap(),
        };
        let claim_refresh: UserClaim = UserClaim {
            sub: ObjectId::to_string(&user._id.unwrap()),
            exp: Utc::now().timestamp() + 259200,
            iss: "Redian".to_string(),
            aud: std::env::var("BASE_URL").unwrap(),
        };

        let header: Header = Header::new(Algorithm::RS256);
        unsafe {
            match (
                encode(
                    &header,
                    &claim_access,
                    &EncodingKey::from_rsa_pem(KEYS.get("private_access").unwrap().as_bytes())
                        .unwrap(),
                ),
                encode(
                    &header,
                    &claim_refresh,
                    &EncodingKey::from_rsa_pem(KEYS.get("private_refresh").unwrap().as_bytes())
                        .unwrap(),
                ),
            ) {
                (Ok(atk), Ok(rtk)) => {
                    let user = User::find_detail_by_id(&user._id.unwrap())
                        .await
                        .map_err(|_| "USER_NOT_FOUND".to_string())?
                        .ok_or("USER_NOT_FOUND")?;
                    Ok((atk, rtk, user))
                }
                _ => Err("GENERATING_FAILED".to_string()),
            }
        }
    }
    pub async fn refresh(token: &str) -> Result<(String, String, UserResponse), String> {
        let validation: Validation = Validation::new(Algorithm::RS256);
        let data: TokenData<UserClaim>;

        unsafe {
            data = decode::<UserClaim>(
                token,
                &DecodingKey::from_rsa_pem(KEYS.get("public_refresh").unwrap().as_bytes()).unwrap(),
                &validation,
            )
            .map_err(|_| "INVALID_TOKEN")?;
        }
        let _id = ObjectId::from_str(&data.claims.sub).map_err(|_| "INVALID_ID".to_string())?;

        let user = User::find_by_id(&_id)
            .await?
            .ok_or_else(|| "USER_NOT_FOUDN".to_string())?;

        let claim_access: UserClaim = UserClaim {
            sub: ObjectId::to_string(&user._id.unwrap()),
            exp: Utc::now().timestamp() + 1800,
            iss: "Redian".to_string(),
            aud: std::env::var("BASE_URL").unwrap(),
        };
        let claim_refresh: UserClaim = UserClaim {
            sub: ObjectId::to_string(&user._id.unwrap()),
            exp: Utc::now().timestamp() + 259200,
            iss: "Redian".to_string(),
            aud: std::env::var("BASE_URL").unwrap(),
        };

        let header: Header = Header::new(Algorithm::RS256);
        unsafe {
            match (
                encode(
                    &header,
                    &claim_access,
                    &EncodingKey::from_rsa_pem(KEYS.get("private_access").unwrap().as_bytes())
                        .unwrap(),
                ),
                encode(
                    &header,
                    &claim_refresh,
                    &EncodingKey::from_rsa_pem(KEYS.get("private_refresh").unwrap().as_bytes())
                        .unwrap(),
                ),
            ) {
                (Ok(atk), Ok(rtk)) => {
                    let user = User::find_detail_by_id(&user._id.unwrap())
                        .await
                        .map_err(|_| "USER_NOT_FOUND".to_string())?
                        .ok_or("USER_NOT_FOUND")?;
                    Ok((atk, rtk, user))
                }
                _ => Err("GENERATING_FAILED".to_string()),
            }
        }
    }
    pub fn verify(token: &str) -> Option<ObjectId> {
        let validation: Validation = Validation::new(Algorithm::RS256);
        unsafe {
            match decode::<UserClaim>(
                token,
                &DecodingKey::from_rsa_pem(KEYS.get("public_access").unwrap().as_bytes()).unwrap(),
                &validation,
            ) {
                Ok(data) => match ObjectId::from_str(&data.claims.sub) {
                    Ok(id) => Some(id),
                    Err(_) => None,
                },
                Err(_) => None,
            }
        }
    }
}

impl<S, B> Service<ServiceRequest> for UserAuthenticationMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    actix_service::forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let srv: Rc<S> = self.service.clone();

        async move {
            let headers: &actix_web::http::header::HeaderMap = req.headers();
            if let Some(bearer_token) = headers.get("Authorization") {
                let mut bytes_token: Vec<u8> = Vec::new();
                for i in bearer_token.as_bytes() {
                    bytes_token.push(*i);
                }
                if bytes_token.len() > 7 {
                    bytes_token.drain(0..7);
                    let token: String = String::from_utf8(bytes_token).unwrap();
                    if let Some(_id) = UserCredential::verify(&token) {
                        if let Ok(Some(user)) = User::find_by_id(&_id).await {
                            let auth_data: UserAuthenticationData = UserAuthenticationData {
                                _id: Some(_id),
                                role_id: user.role_id,
                                token,
                            };
                            req.extensions_mut()
                                .insert::<UserAuthentication>(Rc::new(auth_data));
                        }
                    }
                }
            }
            let res: ServiceResponse<B> = srv.call(req).await?;
            Ok(res)
        }
        .boxed_local()
    }
}
impl<S, B> Transform<S, ServiceRequest> for UserAuthenticationMiddlewareFactory
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = UserAuthenticationMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(UserAuthenticationMiddleware {
            service: Rc::new(service),
        }))
    }
}

pub fn load_keys() {
    let private_access_file =
        read_to_string("./keys/private_access.key").expect("LOAD_FAILED_PRIVATE_ACCESS");
    let public_access_file =
        read_to_string("./keys/public_access.pem").expect("LOAD_FAILED_PUBLIC_ACCESS");
    let private_refresh_file =
        read_to_string("./keys/private_refresh.key").expect("LOAD_FAILED_PRIVATE_ACCESS");
    let public_refresh_file =
        read_to_string("./keys/public_refresh.pem").expect("LOAD_FAILED_PUBLIC_ACCESS");
    unsafe {
        KEYS.insert("private_access".to_string(), private_access_file);
        KEYS.insert("public_access".to_string(), public_access_file);
        KEYS.insert("private_refresh".to_string(), private_refresh_file);
        KEYS.insert("public_refresh".to_string(), public_refresh_file);
    }
}
