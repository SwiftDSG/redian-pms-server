use std::{
    fs::{create_dir_all, remove_dir_all, rename},
    path::PathBuf,
};

use actix_multipart::form::MultipartForm;
use actix_web::{delete, get, post, put, web, HttpMessage, HttpRequest, HttpResponse};
use mime_guess::get_mime_extensions_str;
use mongodb::bson::oid::ObjectId;

use crate::models::{
    customer::{
        Customer, CustomerImage, CustomerImageMultipartRequest, CustomerQuery, CustomerRequest,
    },
    role::{Role, RolePermission},
    user::UserAuthentication,
};

#[get("/customers")]
pub async fn get_customers() -> HttpResponse {
    let query: CustomerQuery = CustomerQuery {
        _id: None,
        name: None,
        limit: None,
    };

    match Customer::find_many(&query).await {
        Ok(Some(customers)) => HttpResponse::Ok().json(customers),
        Ok(None) => HttpResponse::NotFound().json("CUSTOMER_NOT_FOUND"),
        Err(error) => HttpResponse::BadRequest().body(error),
    }
}
#[get("/customers/{customer_id}")]
pub async fn get_customer(customer_id: web::Path<String>) -> HttpResponse {
    let customer_id = match customer_id.parse() {
        Ok(customer_id) => customer_id,
        _ => return HttpResponse::BadRequest().body("INVALID_ID"),
    };

    match Customer::find_by_id(&customer_id).await {
        Ok(Some(customer)) => HttpResponse::Ok().json(customer),
        Ok(None) => HttpResponse::NotFound().body("CUSTOMER_NOT_FOUND"),
        Err(error) => HttpResponse::InternalServerError().body(error),
    }
}
#[post("/customers")]
pub async fn create_customer(
    payload: web::Json<CustomerRequest>,
    req: HttpRequest,
) -> HttpResponse {
    let issuer_role = match req.extensions().get::<UserAuthentication>() {
        Some(issuer) => issuer.role_id.clone(),
        None => return HttpResponse::Unauthorized().body("UNAUTHORIZED"),
    };
    if issuer_role.is_empty()
        || !Role::validate(&issuer_role, &RolePermission::CreateCustomer).await
    {
        return HttpResponse::Unauthorized().body("UNAUTHORIZED");
    }

    let payload: CustomerRequest = payload.into_inner();
    let mut customer: Customer = Customer {
        _id: None,
        name: payload.name,
        field: payload.field,
        contact: payload.contact,
        person: payload.person,
        image: None,
    };
    if let Some(image) = payload.image {
        customer.image = Some(CustomerImage {
            _id: ObjectId::new(),
            extension: image.extension,
        });
    }
    match customer.save().await {
        Ok(id) => HttpResponse::Created().body(id.to_string()),
        Err(error) => HttpResponse::InternalServerError().body(error),
    }
}
#[put("/customers/{customer_id}")]
pub async fn update_customer(
    customer_id: web::Path<String>,
    payload: web::Json<CustomerRequest>,
    req: HttpRequest,
) -> HttpResponse {
    let issuer_role = match req.extensions().get::<UserAuthentication>() {
        Some(issuer) => issuer.role_id.clone(),
        None => return HttpResponse::Unauthorized().body("UNAUTHORIZED"),
    };
    if issuer_role.is_empty()
        || !Role::validate(&issuer_role, &RolePermission::UpdateCustomer).await
    {
        return HttpResponse::Unauthorized().body("UNAUTHORIZED");
    }

    let customer_id = match customer_id.parse() {
        Ok(customer_id) => customer_id,
        _ => return HttpResponse::BadRequest().body("INVALID_ID"),
    };

    if let Ok(Some(customer)) = Customer::find_by_id(&customer_id).await {
        let payload = payload.into_inner();

        if customer.image.is_some() {
            let old_path = format!("./files/customers/{customer_id}",);
            match remove_dir_all(old_path) {
                _ => (),
            };
        }

        let mut customer = Customer {
            _id: Some(customer_id),
            name: payload.name,
            field: payload.field,
            contact: payload.contact,
            person: payload.person,
            image: None,
        };

        if let Some(image) = payload.image {
            customer.image = Some(CustomerImage {
                _id: ObjectId::new(),
                extension: image.extension,
            });
        }

        return match customer.update().await {
            Ok(customer_id) => HttpResponse::Ok().body(customer_id.to_string()),
            Err(error) => HttpResponse::InternalServerError().body(error),
        };
    } else {
        HttpResponse::NotFound().body("CUSTOMER_NOT_FOUND")
    }
}
#[put("/customers/{customer_id}/image")]
pub async fn update_customer_image(
    customer_id: web::Path<String>,
    form: MultipartForm<CustomerImageMultipartRequest>,
    req: HttpRequest,
) -> HttpResponse {
    let issuer_role = match req.extensions().get::<UserAuthentication>() {
        Some(issuer) => issuer.role_id.clone(),
        None => return HttpResponse::Unauthorized().body("UNAUTHORIZED"),
    };
    if issuer_role.is_empty()
        || !Role::validate(&issuer_role, &RolePermission::UpdateCustomer).await
    {
        return HttpResponse::Unauthorized().body("UNAUTHORIZED");
    }

    let customer_id = match customer_id.parse() {
        Ok(customer_id) => customer_id,
        _ => return HttpResponse::BadRequest().body("INVALID_ID"),
    };

    if let Ok(Some(mut customer)) = Customer::find_by_id(&customer_id).await {
        let image = match &customer.image {
            Some(image) => image,
            None => return HttpResponse::BadRequest().body("CUSTOMER_IMAGE_NOT_FOUND"),
        };

        let save_dir = format!("./files/customers/{}/", customer_id);

        if create_dir_all(&save_dir).is_err() {
            return HttpResponse::InternalServerError()
                .body("DIRECTORY_CREATION_FAILED".to_string());
        }

        if let Some(ext) = get_mime_extensions_str(&image.extension) {
            let ext = *ext.first().unwrap();
            let file_path_temp = form.file.file.path();
            let file_path = PathBuf::from(save_dir.to_owned() + &image._id.to_string() + "." + ext);
            if rename(file_path_temp, &file_path).is_ok() {
                customer.image = Some(CustomerImage {
                    _id: image._id,
                    extension: ext.to_string(),
                });

                match customer.update().await {
                    Ok(customer_id) => HttpResponse::Ok().body(customer_id.to_string()),
                    Err(error) => {
                        customer.image = None;
                        if customer.update().await.is_err() {
                            HttpResponse::InternalServerError()
                                .body("CUSTOMER_IMAGE_DELETION_FAILED".to_string())
                        } else {
                            HttpResponse::BadRequest().body(error.to_string())
                        }
                    }
                }
            } else {
                customer.image = None;
                if customer.update().await.is_err() {
                    HttpResponse::InternalServerError()
                        .body("CUSTOMER_IMAGE_DELETION_FAILED".to_string())
                } else {
                    match remove_dir_all(file_path) {
                        _ => HttpResponse::InternalServerError()
                            .body("CUSTOMER_IMAGE_RENAME_FAILED".to_string()),
                    }
                }
            }
        } else {
            customer.image = None;
            if customer.update().await.is_err() {
                HttpResponse::InternalServerError()
                    .body("CUSTOMER_IMAGE_DELETION_FAILED".to_string())
            } else {
                HttpResponse::InternalServerError().body("CUSTOMER_IMAGE_INVALID_MIME".to_string())
            }
        }
    } else {
        HttpResponse::NotFound().body("CUSTOMER_NOT_FOUND")
    }
}
#[delete("/customers/{customer_id}")]
pub async fn delete_customer(customer_id: web::Path<String>, req: HttpRequest) -> HttpResponse {
    let issuer_role = match req.extensions().get::<UserAuthentication>() {
        Some(issuer) => issuer.role_id.clone(),
        None => return HttpResponse::Unauthorized().body("UNAUTHORIZED"),
    };
    if issuer_role.is_empty()
        || !Role::validate(&issuer_role, &RolePermission::DeleteCustomer).await
    {
        return HttpResponse::Unauthorized().body("UNAUTHORIZED");
    }

    let customer_id = match customer_id.parse() {
        Ok(customer_id) => customer_id,
        _ => return HttpResponse::BadRequest().body("INVALID_ID"),
    };

    if let Ok(Some(customer)) = Customer::find_by_id(&customer_id).await {
        match customer.delete().await {
            Ok(count) => HttpResponse::Ok().body(format!("Deleted {count} customer")),
            Err(error) => HttpResponse::InternalServerError().body(error),
        }
    } else {
        HttpResponse::NotFound().body("CUSTOMER_NOT_FOUND")
    }
}
