use actix_web::{get, post, web, HttpMessage, HttpRequest, HttpResponse};
use mongodb::bson::oid::ObjectId;
use std::str::FromStr;

use crate::models::{
    customer::{Customer, CustomerQuery, CustomerRequest},
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
        Ok(customers) => HttpResponse::Ok().json(customers),
        Err(error) => HttpResponse::BadRequest().body(error),
    }
}
#[get("/customers/{_id}")]
pub async fn get_customer(_id: web::Path<String>) -> HttpResponse {
    let _id: String = _id.into_inner();
    if let Ok(_id) = ObjectId::from_str(&_id) {
        return match Customer::find_by_id(&_id).await {
            Ok(Some(customer)) => HttpResponse::Ok().json(customer),
            Ok(None) => HttpResponse::NotFound().body("CUSTOMER_NOT_FOUND".to_string()),
            Err(error) => HttpResponse::InternalServerError().body(error),
        };
    } else {
        HttpResponse::BadRequest().body("INVALID_ID".to_string())
    }
}
#[post("/customers")]
pub async fn create_customer(
    payload: web::Json<CustomerRequest>,
    req: HttpRequest,
) -> HttpResponse {
    let payload: CustomerRequest = payload.into_inner();
    let mut customer: Customer = Customer {
        _id: None,
        name: payload.name,
        contact: payload.contact,
        person: payload.person,
    };
    if let Some(issuer) = req.extensions().get::<UserAuthentication>().cloned() {
        if !Role::validate(&issuer.role, &RolePermission::CreateCustomer).await {
            return HttpResponse::Unauthorized().body("UNAUTHORIZED".to_string());
        }
    } else {
        return HttpResponse::Unauthorized().body("UNAUTHORIZED".to_string());
    }
    // if let Some(roles) = payload.role {
    //     for i in roles.iter() {
    //         if let Ok(_id) = ObjectId::from_str(i) {
    //             if let Ok(Some(_)) = Role::find_by_id(&_id).await {
    //                 user.role.push(_id);
    //             }
    //         }
    //     }
    // } else {
    //     return HttpResponse::BadRequest().body("USER_MUST_HAVE_ROLES".to_string());
    // }

    match customer.save().await {
        Ok(id) => HttpResponse::Created().body(id.to_string()),
        Err(error) => HttpResponse::InternalServerError().body(error),
    }
}
//+validasi get 1 per 1, update, delete,
