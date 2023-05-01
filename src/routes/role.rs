use actix_web::{delete, get, post, web, HttpMessage, HttpRequest, HttpResponse};
use mongodb::bson::oid::ObjectId;
use std::str::FromStr;

use crate::models::{
    role::{Role, RoleQuery, RoleRequest},
    user::UserAuthentication,
};

#[get("/roles")]
pub async fn get_roles() -> HttpResponse {
    let query: RoleQuery = RoleQuery {
        _id: None,
        limit: None,
    };

    match Role::find_many(&query).await {
        Ok(roles) => HttpResponse::Ok().json(roles),
        Err(error) => HttpResponse::BadRequest().body(error),
    }
}
#[get("/roles/{_id}")]
pub async fn get_role(_id: web::Path<String>) -> HttpResponse {
    let _id: String = _id.into_inner();
    if let Ok(_id) = ObjectId::from_str(&_id) {
        return match Role::find_by_id(&_id).await {
            Ok(Some(role)) => HttpResponse::Ok().json(role),
            Ok(None) => HttpResponse::NotFound().body("ROLE_NOT_FOUND".to_string()),
            Err(error) => HttpResponse::InternalServerError().body(error),
        };
    } else {
        HttpResponse::BadRequest().body("INVALID_ID".to_string())
    }
}
#[delete("/roles/{_id}")]
pub async fn delete_role(_id: web::Path<String>) -> HttpResponse {
    let _id: String = _id.into_inner();
    if let Ok(_id) = ObjectId::from_str(&_id) {
        return match Role::delete_by_id(&_id).await {
            Ok(count) => HttpResponse::Ok().body(format!("Deleted {count} role")),
            Err(error) => HttpResponse::InternalServerError().body(error),
        };
    } else {
        HttpResponse::BadRequest().body("INVALID_ID".to_string())
    }
}
#[post("/roles")]
pub async fn create_role(payload: web::Json<RoleRequest>, req: HttpRequest) -> HttpResponse {
    if let Some(issuer) = req.extensions().get::<UserAuthentication>() {
        if !Role::validate(&issuer.role, &"add_role".to_string()).await {
            let payload = payload.into_inner();

            let mut role = Role {
                _id: None,
                name: payload.name,
                permission: Vec::<String>::new(),
            };

            for i in payload.permission.iter() {
                role.add_permission(i);
            }

            if role.permission.is_empty() {
                return HttpResponse::BadRequest()
                    .body("ROLE_MUST_HAVE_VALID_PERMISSION".to_string());
            }

            match role.save().await {
                Ok(id) => HttpResponse::Created().body(id.to_string()),
                Err(error) => HttpResponse::InternalServerError().body(error),
            }
        } else {
            HttpResponse::Unauthorized().body("UNAUTHORIZED".to_string())
        }
    } else {
        HttpResponse::Unauthorized().body("UNAUTHORIZED".to_string())
    }
}
