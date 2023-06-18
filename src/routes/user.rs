use actix_web::{get, post, web, HttpMessage, HttpRequest, HttpResponse};
use mongodb::bson::{doc, oid::ObjectId, to_bson};
use regex::Regex;
use std::str::FromStr;

use crate::models::{
    role::{Role, RolePermission},
    user::{User, UserAuthentication, UserCredential, UserQuery, UserRefresh, UserRequest},
};

#[get("/users")]
pub async fn get_users() -> HttpResponse {
    let query: UserQuery = UserQuery {
        _id: None,
        email: None,
        limit: None,
    };

    match User::find_many(&query).await {
        Ok(users) => HttpResponse::Ok().json(users),
        Err(error) => HttpResponse::BadRequest().body(error),
    }
}
#[get("/users/{_id}")]
pub async fn get_user(_id: web::Path<String>) -> HttpResponse {
    let _id: String = _id.into_inner();
    if let Ok(_id) = ObjectId::from_str(&_id) {
        return match User::find_by_id(&_id).await {
            Ok(Some(user)) => HttpResponse::Ok().json(user),
            Ok(None) => HttpResponse::NotFound().body("USER_NOT_FOUND".to_string()),
            Err(error) => HttpResponse::InternalServerError().body(error),
        };
    } else {
        HttpResponse::BadRequest().body("INVALID_ID".to_string())
    }
}
#[post("/users")]
pub async fn create_user(payload: web::Json<UserRequest>, req: HttpRequest) -> HttpResponse {
    let payload: UserRequest = payload.into_inner();
    let email_regex: Regex = Regex::new(
        r"^([a-z0-9_+]([a-z0-9_+.]*[a-z0-9_+])?)@([a-z0-9]+([\-\.]{1}[a-z0-9]+)*\.[a-z]{2,6})",
    )
    .unwrap();

    if payload.password.len() < 8 {
        return HttpResponse::BadRequest().body("USER_MUST_HAVE_VALID_PASSWORD".to_string());
    }
    if !email_regex.is_match(&payload.email) {
        return HttpResponse::BadRequest().body("USER_MUST_HAVE_VALID_EMAIL".to_string());
    }

    let mut user: User = User {
        _id: None,
        role_id: Vec::<ObjectId>::new(),
        name: payload.name,
        email: payload.email,
        password: payload.password,
        image: None,
    };

    if (User::find_many(&UserQuery {
        _id: None,
        email: None,
        limit: Some(1),
    })
    .await)
        .is_ok()
    {
        let issuer_role = match req.extensions().get::<UserAuthentication>() {
            Some(issuer) => issuer.role_id.clone(),
            None => return HttpResponse::Unauthorized().body("UNAUTHORIZED".to_string()),
        };
        if issuer_role.is_empty()
            || !Role::validate(&issuer_role, &RolePermission::CreateUser).await
        {
            return HttpResponse::Unauthorized().body("UNAUTHORIZED".to_string());
        }

        if let Some(roles) = payload.role_id {
            for i in roles.iter() {
                if let Ok(Some(_)) = Role::find_by_id(i).await {
                    user.role_id.push(*i);
                }
            }
        } else {
            return HttpResponse::BadRequest().body("USER_MUST_HAVE_ROLES".to_string());
        }
    } else {
        if (Role::delete_many().await).is_err() {
            return HttpResponse::BadRequest().body("UNABLE_TO_DELETE_ROLES".to_string());
        }
        let mut role: Role = Role {
            _id: None,
            name: "Owner".to_string(),
            permission: Vec::<RolePermission>::new(),
        };
        role.set_as_owner();
        if let Ok(_id) = role.save().await {
            user.role_id = vec![_id];
        } else {
            return HttpResponse::BadRequest().body("UNABLE_TO_CREATE_ROLE".to_string());
        }
    }

    if let Ok(Some(_)) = User::find_by_email(&user.email).await {
        HttpResponse::BadRequest().body("USER_ALREADY_EXIST".to_string())
    } else {
        match user.save().await {
            Ok(id) => HttpResponse::Created().body(id.to_string()),
            Err(error) => HttpResponse::InternalServerError().body(error),
        }
    }
}
#[post("/users/login")]
pub async fn login(payload: web::Json<UserCredential>) -> HttpResponse {
    let payload: UserCredential = payload.into_inner();

    match payload.authenticate().await {
        Ok((atk, rtk)) => HttpResponse::Ok().json(doc! {
            "atk": to_bson::<String>(&atk).unwrap(),
            "rtk": to_bson::<String>(&rtk).unwrap()
        }),
        Err(error) => HttpResponse::InternalServerError().body(error),
    }
}
#[post("/users/refresh")]
pub async fn refresh(payload: web::Json<UserRefresh>) -> HttpResponse {
    let payload: UserRefresh = payload.into_inner();

    match UserCredential::refresh(&payload.rtk).await {
        Ok((atk, rtk)) => HttpResponse::Ok().json(doc! {
            "atk": to_bson::<String>(&atk).unwrap(),
            "rtk": to_bson::<String>(&rtk).unwrap()
        }),
        Err(error) => HttpResponse::InternalServerError().body(error),
    }
}
