use crate::database::get_db;
use actix_service::{self, Transform};
use actix_web::{
    dev::{Service, ServiceRequest, ServiceResponse},
    Error, HttpMessage,
};
use chrono::Utc;
use futures::{
    future::{ready, LocalBoxFuture, Ready},
    stream::TryStreamExt,
    FutureExt,
};
use jsonwebtoken::{self, decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use mongodb::{
    bson::{doc, oid::ObjectId},
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
#[derive(Clone, Debug, Deserialize, Serialize, Default)]
pub struct UserAuthenticationData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _id: Option<ObjectId>,
    pub role: String,
    pub token: String,
}
pub struct UserAuthenticationMiddleware<S> {
    service: Rc<S>,
}
pub struct UserAuthenticationMiddlewareFactory;

pub type UserAuthentication = Rc<UserAuthenticationData>;

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
    pub fn verify(token: &String) -> Option<ObjectId> {
        let validation: Validation = Validation::new(Algorithm::RS256);
        unsafe {
            if let Ok(data) = decode::<UserClaims>(
                token,
                &DecodingKey::from_rsa_pem(&KEYS.get("public_access").unwrap().as_bytes()).unwrap(),
                &validation,
            ) {
                if let Ok(_id) = ObjectId::from_str(&data.claims.sub) {
                    return Some(_id);
                } else {
                    return None;
                }
            } else {
                None
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
                bytes_token.drain(0..7);
                let token: String = String::from_utf8(bytes_token).unwrap();
                if let Some(_id) = UserCredential::verify(&token) {
                    if let Ok(Some(user)) = User::find_by_id(&_id).await {
                        let auth_data: UserAuthenticationData = UserAuthenticationData {
                            _id: Some(_id),
                            role: user.role,
                            token,
                        };
                        req.extensions_mut()
                            .insert::<UserAuthentication>(Rc::new(auth_data));
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
    unsafe {
        KEYS.insert(String::from("private_access"), private_access_file);
        KEYS.insert(String::from("public_access"), public_access_file);
    }
}
