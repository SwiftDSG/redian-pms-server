use std::{
    fs::{create_dir_all, remove_dir_all, rename},
    path::PathBuf,
};

use actix_multipart::form::MultipartForm;
use actix_web::{get, post, put, web, HttpMessage, HttpRequest, HttpResponse};
use mime_guess::get_mime_extensions_str;
use mongodb::bson::oid::ObjectId;

use crate::models::{
    company::{Company, CompanyImage, CompanyImageMultipartRequest, CompanyRequest},
    role::{Role, RolePermission},
    user::UserAuthentication,
};

#[get("/companies")]
pub async fn get_company() -> HttpResponse {
    match Company::find_detail().await {
        Ok(Some(company)) => HttpResponse::Ok().json(company),
        Ok(None) => HttpResponse::NotFound().body("COMPANY_NOT_FOUND"),
        Err(error) => HttpResponse::InternalServerError().body(error),
    }
}
#[post("/companies")]
pub async fn create_company(payload: web::Json<CompanyRequest>, req: HttpRequest) -> HttpResponse {
    let issuer_role = match req.extensions().get::<UserAuthentication>() {
        Some(issuer) => issuer.role_id.clone(),
        None => return HttpResponse::Unauthorized().body("UNAUTHORIZED"),
    };
    if issuer_role.is_empty() || !Role::validate(&issuer_role, &RolePermission::Owner).await {
        return HttpResponse::Unauthorized().body("UNAUTHORIZED");
    }

    let payload: CompanyRequest = payload.into_inner();
    let mut company: Company = Company {
        _id: None,
        name: payload.name,
        field: payload.field,
        contact: payload.contact,
        image: None,
    };

    if let Some(image) = payload.image {
        company.image = Some(CompanyImage {
            _id: ObjectId::new(),
            extension: image.extension,
        });
    }

    match company.save().await {
        Ok(id) => HttpResponse::Created().body(id.to_string()),
        Err(error) => HttpResponse::InternalServerError().body(error),
    }
}
#[put("/companies/{company_id}")]
pub async fn update_company(
    company_id: web::Path<String>,
    payload: web::Json<CompanyRequest>,
    req: HttpRequest,
) -> HttpResponse {
    let issuer_role = match req.extensions().get::<UserAuthentication>() {
        Some(issuer) => issuer.role_id.clone(),
        None => return HttpResponse::Unauthorized().body("UNAUTHORIZED"),
    };
    if issuer_role.is_empty() || !Role::validate(&issuer_role, &RolePermission::Owner).await {
        return HttpResponse::Unauthorized().body("UNAUTHORIZED");
    }

    let company_id = match company_id.parse() {
        Ok(company_id) => company_id,
        _ => return HttpResponse::BadRequest().body("INVALID_ID"),
    };

    if let Ok(Some(mut company)) = Company::find_by_id(&company_id).await {
        let payload = payload.into_inner();

        if let Some(_) = &company.image {
            let old_path = format!("./files/companies/{company_id}",);
            remove_dir_all(old_path).expect("COMPANY_IMAGE_DELETION_FAILED");
        }
        company = Company {
            _id: Some(company_id),
            name: payload.name,
            field: payload.field,
            contact: payload.contact,
            image: None,
        };

        if let Some(image) = payload.image {
            company.image = Some(CompanyImage {
                _id: ObjectId::new(),
                extension: image.extension,
            });
        }

        return match company.update().await {
            Ok(company_id) => HttpResponse::Ok().body(company_id.to_string()),
            Err(error) => HttpResponse::InternalServerError().body(error),
        };
    } else {
        HttpResponse::NotFound().body("COMPANY_NOT_FOUND")
    }
}
#[put("/companies/{company_id}/image")]
pub async fn update_company_image(
    company_id: web::Path<String>,
    form: MultipartForm<CompanyImageMultipartRequest>,
    req: HttpRequest,
) -> HttpResponse {
    let issuer_role = match req.extensions().get::<UserAuthentication>() {
        Some(issuer) => issuer.role_id.clone(),
        None => return HttpResponse::Unauthorized().body("UNAUTHORIZED"),
    };
    if issuer_role.is_empty() || !Role::validate(&issuer_role, &RolePermission::Owner).await {
        return HttpResponse::Unauthorized().body("UNAUTHORIZED");
    }

    let company_id = match company_id.parse() {
        Ok(company_id) => company_id,
        _ => return HttpResponse::BadRequest().body("INVALID_ID"),
    };

    if let Ok(Some(mut company)) = Company::find_by_id(&company_id).await {
        let image = match &company.image {
            Some(image) => image,
            None => return HttpResponse::BadRequest().body("COMPANY_IMAGE_NOT_FOUND"),
        };

        let save_dir = format!("./files/companies/{}/", company_id);

        if create_dir_all(&save_dir).is_err() {
            return HttpResponse::InternalServerError()
                .body("DIRECTORY_CREATION_FAILED".to_string());
        }

        if let Some(ext) = get_mime_extensions_str(&image.extension) {
            let ext = *ext.first().unwrap();
            let file_path_temp = form.file.file.path();
            let file_path =
                PathBuf::from(save_dir.to_owned() + &image._id.to_string() + "." + &ext);
            if rename(file_path_temp, &file_path).is_ok() {
                company.image = Some(CompanyImage {
                    _id: image._id,
                    extension: ext.to_string(),
                });

                match company.update().await {
                    Ok(company_id) => HttpResponse::Ok().body(company_id.to_string()),
                    Err(error) => {
                        company.image = None;
                        company
                            .update()
                            .await
                            .expect("COMPANY_IMAGE_DELETION_FAILED");
                        HttpResponse::BadRequest().body(error.to_string())
                    }
                }
            } else {
                company.image = None;
                remove_dir_all(file_path).expect("COMPANY_IMAGE_DELETION_FAILED");
                company
                    .update()
                    .await
                    .expect("COMPANY_IMAGE_DELETION_FAILED");
                HttpResponse::InternalServerError().body("COMPANY_IMAGE_RENAME_FAILED".to_string())
            }
        } else {
            company.image = None;
            company
                .update()
                .await
                .expect("COMPANY_IMAGE_DELETION_FAILED");
            HttpResponse::InternalServerError().body("COMPANY_IMAGE_INVALID_MIME".to_string())
        }
    } else {
        HttpResponse::NotFound().body("COMPANY_NOT_FOUND")
    }
}
