use actix_web::{delete, get, post, put, web, HttpMessage, HttpRequest, HttpResponse};

use crate::models::{
    role::{Role, RolePermission, RoleQuery, RoleRequest},
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
#[post("/roles")]
pub async fn create_role(payload: web::Json<RoleRequest>, req: HttpRequest) -> HttpResponse {
    let issuer_role = match req.extensions().get::<UserAuthentication>() {
        Some(issuer) => issuer.role_id.clone(),
        None => return HttpResponse::Unauthorized().body("UNAUTHORIZED".to_string()),
    };
    if issuer_role.is_empty() || !Role::validate(&issuer_role, &RolePermission::CreateRole).await {
        return HttpResponse::Unauthorized().body("UNAUTHORIZED".to_string());
    }

    let payload: RoleRequest = payload.into_inner();

    let mut role: Role = Role {
        _id: None,
        name: payload.name,
        permission: payload.permission,
    };

    if role.permission.contains(&RolePermission::Owner) {
        return HttpResponse::BadRequest().body("ROLE_MUST_HAVE_VALID_PERMISSION".to_string());
    }

    match role.save().await {
        Ok(_id) => HttpResponse::Created().body(_id.to_string()),
        Err(error) => HttpResponse::InternalServerError().body(error),
    }
}
#[get("/roles/{role_id}")]
pub async fn get_role(role_id: web::Path<String>) -> HttpResponse {
    let role_id = match role_id.parse() {
        Ok(role_id) => role_id,
        _ => return HttpResponse::BadRequest().body("INVALID_ID".to_string()),
    };

    return match Role::find_by_id(&role_id).await {
        Ok(Some(role)) => HttpResponse::Ok().json(role),
        Ok(None) => HttpResponse::NotFound().body("ROLE_NOT_FOUND".to_string()),
        Err(error) => HttpResponse::InternalServerError().body(error),
    };
}
#[delete("/roles/{role_id}")]
pub async fn delete_role(role_id: web::Path<String>, req: HttpRequest) -> HttpResponse {
    let role_id = match role_id.parse() {
        Ok(role_id) => role_id,
        _ => return HttpResponse::BadRequest().body("INVALID_ID".to_string()),
    };

    let issuer_role = match req.extensions().get::<UserAuthentication>() {
        Some(issuer) => issuer.role_id.clone(),
        None => return HttpResponse::Unauthorized().body("UNAUTHORIZED".to_string()),
    };
    if issuer_role.is_empty() || !Role::validate(&issuer_role, &RolePermission::DeleteRole).await {
        return HttpResponse::Unauthorized().body("UNAUTHORIZED".to_string());
    }

    return match Role::delete_by_id(&role_id).await {
        Ok(count) => HttpResponse::Ok().body(format!("Deleted {count} role")),
        Err(error) => HttpResponse::InternalServerError().body(error),
    };
}
#[put("/roles/{role_id}")]
pub async fn update_role(
    role_id: web::Path<String>,
    payload: web::Json<RoleRequest>,
    req: HttpRequest,
) -> HttpResponse {
    let role_id = match role_id.parse() {
        Ok(role_id) => role_id,
        _ => return HttpResponse::BadRequest().body("INVALID_ID".to_string()),
    };

    let issuer_role = match req.extensions().get::<UserAuthentication>() {
        Some(issuer) => issuer.role_id.clone(),
        None => return HttpResponse::Unauthorized().body("UNAUTHORIZED".to_string()),
    };
    if issuer_role.is_empty() || !Role::validate(&issuer_role, &RolePermission::UpdateRole).await {
        return HttpResponse::Unauthorized().body("UNAUTHORIZED".to_string());
    }

    let payload: RoleRequest = payload.into_inner();

    if let Ok(Some(mut role)) = Role::find_by_id(&role_id).await {
        role.name = payload.name;
        role.permission = payload.permission;

        if role.permission.contains(&RolePermission::Owner) {
            return HttpResponse::BadRequest().body("ROLE_MUST_HAVE_VALID_PERMISSION".to_string());
        }

        match role.update().await {
            Ok(_id) => HttpResponse::Ok().body(_id.to_string()),
            Err(error) => HttpResponse::InternalServerError().body(error),
        }
    } else {
        HttpResponse::BadRequest().body("ROLE_NOT_FOUND")
    }
}
