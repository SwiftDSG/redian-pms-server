use actix_web::{get, web, HttpResponse};
use mongodb::bson::oid::ObjectId;
use std::str::FromStr;

use crate::models::user::User;

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
        return match User::find_one(_id).await {
            Ok(Some(user)) => HttpResponse::Ok().json(user),
            Ok(None) => HttpResponse::NotFound().body(format!("USER_NOT_FOUND")),
            Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
        };
    } else {
        HttpResponse::BadRequest().body(format!("INVALID_ID"))
    }
}
