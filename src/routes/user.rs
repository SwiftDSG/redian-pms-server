use std::{
    fs::{create_dir_all, remove_dir_all, rename},
    path::PathBuf,
};

use actix_multipart::form::MultipartForm;
use actix_web::{get, post, put, web, HttpMessage, HttpRequest, HttpResponse};
use mime_guess::get_mime_extensions_str;
use mongodb::bson::{doc, oid::ObjectId, to_bson};
use regex::Regex;

use crate::models::{
    role::{Role, RolePermission},
    user::{
        User, UserAuthentication, UserCredential, UserImage, UserImageMultipartRequest, UserQuery,
        UserRefreshRequest, UserRequest, UserResponse,
    },
};

#[get("/users")]
pub async fn get_users() -> HttpResponse {
    let query: UserQuery = UserQuery {
        _id: None,
        role_id: None,
        email: None,
        limit: None,
    };

    match User::find_many(&query).await {
        Ok(users) => HttpResponse::Ok().json(users),
        Err(error) => HttpResponse::BadRequest().body(error),
    }
}
#[get("/users/{user_id}")]
pub async fn get_user(user_id: web::Path<String>) -> HttpResponse {
    let user_id = match user_id.parse() {
        Ok(user_id) => user_id,
        Err(_) => return HttpResponse::BadRequest().body("INVALID_ID"),
    };

    match User::find_detail_by_id(&user_id).await {
        Ok(Some(user)) => HttpResponse::Ok().json(user),
        Ok(None) => HttpResponse::NotFound().body("USER_NOT_FOUND"),
        Err(error) => HttpResponse::InternalServerError().body(error),
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
        return HttpResponse::BadRequest().body("USER_MUST_HAVE_VALID_PASSWORD");
    }
    if !email_regex.is_match(&payload.email) {
        return HttpResponse::BadRequest().body("USER_MUST_HAVE_VALID_EMAIL");
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
        role_id: None,
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
        match Role::delete_many().await {
            Ok(_) => (),
            Err(error) => return HttpResponse::InternalServerError().body(error),
        }
        if Role::delete_many().await.is_err() {
            return HttpResponse::InternalServerError().body("UNABLE_TO_DELETE_ROLES");
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
            return HttpResponse::BadRequest().body("UNABLE_TO_CREATE_ROLE");
        }
    }

    if let Ok(Some(_)) = User::find_by_email(&user.email).await {
        HttpResponse::BadRequest().body("USER_ALREADY_EXIST")
    } else {
        match user.save().await {
            Ok(id) => HttpResponse::Created().body(id.to_string()),
            Err(error) => HttpResponse::InternalServerError().body(error),
        }
    }
}
#[put("/users/{user_id}")]
pub async fn update_user(
    user_id: web::Path<String>,
    payload: web::Json<UserRequest>,
    req: HttpRequest,
) -> HttpResponse {
    let issuer_role = match req.extensions().get::<UserAuthentication>() {
        Some(issuer) => issuer.role_id.clone(),
        None => return HttpResponse::Unauthorized().body("UNAUTHORIZED"),
    };
    if issuer_role.is_empty() || !Role::validate(&issuer_role, &RolePermission::UpdateUser).await {
        return HttpResponse::Unauthorized().body("UNAUTHORIZED");
    }

    let user_id = match user_id.parse() {
        Ok(user_id) => user_id,
        _ => return HttpResponse::BadRequest().body("INVALID_ID"),
    };

    if let Ok(Some(user)) = User::find_by_id(&user_id).await {
        let payload = payload.into_inner();
        let mut update_hash = false;

        if user.image.is_some() {
            let old_path = format!("./files/users/{user_id}",);
            match remove_dir_all(old_path) {
                _ => (),
            };
        }

        let mut user = User {
            _id: Some(user_id),
            role_id: issuer_role,
            name: payload.name,
            email: payload.email,
            password: user.password,
            image: None,
        };

        if payload.password != *"*" {
            update_hash = true;
            user.password = payload.password;
        }

        if let Some(image) = payload.image {
            user.image = Some(UserImage {
                _id: ObjectId::new(),
                extension: image.extension,
            });
        }

        return match user.update(update_hash).await {
            Ok(user_id) => HttpResponse::Ok().body(user_id.to_string()),
            Err(error) => HttpResponse::InternalServerError().body(error),
        };
    } else {
        HttpResponse::NotFound().body("USER_NOT_FOUND")
    }
}
#[put("/users/{user_id}/image")]
pub async fn update_user_image(
    user_id: web::Path<String>,
    form: MultipartForm<UserImageMultipartRequest>,
    req: HttpRequest,
) -> HttpResponse {
    let issuer_role = match req.extensions().get::<UserAuthentication>() {
        Some(issuer) => issuer.role_id.clone(),
        None => return HttpResponse::Unauthorized().body("UNAUTHORIZED"),
    };
    if issuer_role.is_empty() || !Role::validate(&issuer_role, &RolePermission::UpdateUser).await {
        return HttpResponse::Unauthorized().body("UNAUTHORIZED");
    }

    let user_id = match user_id.parse() {
        Ok(user_id) => user_id,
        _ => return HttpResponse::BadRequest().body("INVALID_ID"),
    };

    if let Ok(Some(mut user)) = User::find_by_id(&user_id).await {
        let image = match &user.image {
            Some(image) => image,
            None => return HttpResponse::BadRequest().body("USER_IMAGE_NOT_FOUND"),
        };

        let save_dir = format!("./files/users/{}/", user_id);

        if create_dir_all(&save_dir).is_err() {
            return HttpResponse::InternalServerError()
                .body("DIRECTORY_CREATION_FAILED".to_string());
        }

        if let Some(ext) = get_mime_extensions_str(&image.extension) {
            let ext = *ext.first().unwrap();
            let file_path_temp = form.file.file.path();
            let file_path = PathBuf::from(save_dir.to_owned() + &image._id.to_string() + "." + ext);
            if rename(file_path_temp, &file_path).is_ok() {
                user.image = Some(UserImage {
                    _id: image._id,
                    extension: ext.to_string(),
                });

                match user.update(false).await {
                    Ok(user_id) => HttpResponse::Ok().body(user_id.to_string()),
                    Err(error) => {
                        user.image = None;
                        if user.update(false).await.is_err() {
                            HttpResponse::InternalServerError()
                                .body("USER_IMAGE_DELETION_FAILED".to_string())
                        } else {
                            HttpResponse::BadRequest().body(error.to_string())
                        }
                    }
                }
            } else {
                user.image = None;
                if user.update(false).await.is_err() {
                    HttpResponse::InternalServerError()
                        .body("USER_IMAGE_DELETION_FAILED".to_string())
                } else {
                    match remove_dir_all(file_path) {
                        _ => HttpResponse::InternalServerError()
                            .body("USER_IMAGE_RENAME_FAILED".to_string()),
                    }
                }
            }
        } else {
            user.image = None;
            if user.update(false).await.is_err() {
                HttpResponse::InternalServerError().body("USER_IMAGE_DELETION_FAILED".to_string())
            } else {
                HttpResponse::InternalServerError().body("USER_IMAGE_INVALID_MIME".to_string())
            }
        }
    } else {
        HttpResponse::NotFound().body("USER_NOT_FOUND")
    }
}
#[post("/users/login")]
pub async fn login(payload: web::Json<UserCredential>) -> HttpResponse {
    let payload: UserCredential = payload.into_inner();

    match payload.authenticate().await {
        Ok((atk, rtk, user)) => HttpResponse::Ok().json(doc! {
            "atk": to_bson::<String>(&atk).unwrap(),
            "rtk": to_bson::<String>(&rtk).unwrap(),
            "user": to_bson::<UserResponse>(&user).unwrap()
        }),
        Err(error) => HttpResponse::InternalServerError().body(error),
    }
}
#[post("/users/refresh")]
pub async fn refresh(payload: web::Json<UserRefreshRequest>) -> HttpResponse {
    let payload: UserRefreshRequest = payload.into_inner();

    match UserCredential::refresh(&payload.rtk).await {
        Ok((atk, rtk, user)) => HttpResponse::Ok().json(doc! {
            "atk": to_bson::<String>(&atk).unwrap(),
            "rtk": to_bson::<String>(&rtk).unwrap(),
            "user": to_bson::<UserResponse>(&user).unwrap()
        }),
        Err(error) => HttpResponse::InternalServerError().body(error),
    }
}
