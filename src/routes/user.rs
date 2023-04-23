use actix_web::{get, post, web, HttpResponse};
use mongodb::bson::oid::ObjectId;
use std::str::FromStr;

use crate::models::user::{User, UserCredential};

#[get("/users")]
pub async fn get_users() -> HttpResponse {
    let results = User::find_many().await.unwrap();
    println!("{:#?}", results);

    HttpResponse::Ok().json(results)
}
#[get("/users/{_id}")]
pub async fn get_user(_id: web::Path<String>) -> HttpResponse {
    let _id: String = _id.into_inner();
    if let Ok(_id) = ObjectId::from_str(&_id) {
        return match User::find_by_id(&_id).await {
            Ok(Some(user)) => HttpResponse::Ok().json(user),
            Ok(None) => HttpResponse::NotFound().body(format!("USER_NOT_FOUND")),
            Err(error) => HttpResponse::InternalServerError().body(format!("{error}")),
        };
    } else {
        HttpResponse::BadRequest().body(format!("INVALID_ID"))
    }
}
#[post("/users")]
pub async fn create_user(payload: web::Json<User>) -> HttpResponse {
    let payload: User = payload.into_inner();

    if let Ok(Some(_)) = User::find_by_email(&payload.email).await {
        HttpResponse::BadRequest().body(format!("USER_ALREADY_EXIST"))
    } else {
        match payload.save().await {
            Ok(id) => HttpResponse::Created().body(format!("{id}")),
            Err(error) => HttpResponse::InternalServerError().body(format!("{error}")),
        }
    }
}
#[post("/users/login")]
pub async fn login(payload: web::Json<UserCredential>) -> HttpResponse {
    // let x = req.extensions().get::<UserAuthentication>().cloned();
    let payload: UserCredential = payload.into_inner();

    match payload.authenticate().await {
        Ok(token) => HttpResponse::Ok().body(format!("{token}")),
        Err(error) => HttpResponse::InternalServerError().body(format!("{error}")),
    }
}
