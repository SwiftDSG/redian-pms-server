use actix_web::{delete, get, post, put, web, HttpMessage, HttpRequest, HttpResponse};

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
#[get("/customers/{customer_id}")]
pub async fn get_customer(customer_id: web::Path<String>) -> HttpResponse {
    let customer_id = match customer_id.parse() {
        Ok(customer_id) => customer_id,
        _ => return HttpResponse::BadRequest().body("INVALID_ID".to_string()),
    };

    match Customer::find_by_id(&customer_id).await {
        Ok(Some(customer)) => HttpResponse::Ok().json(customer),
        Ok(None) => HttpResponse::NotFound().body("CUSTOMER_NOT_FOUND".to_string()),
        Err(error) => HttpResponse::InternalServerError().body(error),
    }
}
#[post("/customers")]
pub async fn create_customer(
    payload: web::Json<CustomerRequest>,
    req: HttpRequest,
) -> HttpResponse {
    let issuer_role = match req.extensions().get::<UserAuthentication>() {
        Some(issuer) => issuer.role.clone(),
        None => return HttpResponse::Unauthorized().body("UNAUTHORIZED".to_string()),
    };
    if issuer_role.is_empty()
        || !Role::validate(&issuer_role, &RolePermission::CreateCustomer).await
    {
        return HttpResponse::Unauthorized().body("UNAUTHORIZED".to_string());
    }

    let payload: CustomerRequest = payload.into_inner();
    let mut customer: Customer = Customer {
        _id: None,
        name: payload.name,
        contact: payload.contact,
        person: payload.person,
    };
    match customer.save().await {
        Ok(id) => HttpResponse::Created().body(id.to_string()),
        Err(error) => HttpResponse::InternalServerError().body(error),
    }
}
#[put("/customers/{customer_id}")]
pub async fn update_customer(
    customer_id: web::Path<String>,
    payload: web::Json<Customer>,
    req: HttpRequest,
) -> HttpResponse {
    let issuer_role = match req.extensions().get::<UserAuthentication>() {
        Some(issuer) => issuer.role.clone(),
        None => return HttpResponse::Unauthorized().body("UNAUTHORIZED".to_string()),
    };
    if issuer_role.is_empty()
        || !Role::validate(&issuer_role, &RolePermission::UpdateCustomer).await
    {
        return HttpResponse::Unauthorized().body("UNAUTHORIZED".to_string());
    }

    let customer_id = match customer_id.parse() {
        Ok(customer_id) => customer_id,
        _ => return HttpResponse::BadRequest().body("INVALID_ID".to_string()),
    };

    if let Ok(Some(_)) = Customer::find_by_id(&customer_id).await {
        let payload: Customer = payload.into_inner();
        let mut customer: Customer = Customer {
            _id: Some(customer_id),
            name: payload.name,
            contact: payload.contact,
            person: payload.person,
        };
        return match customer.update_customer().await {
            Ok(customer_id) => HttpResponse::Ok().body(customer_id.to_string()),
            Err(error) => HttpResponse::InternalServerError().body(error),
        };
    } else {
        HttpResponse::NotFound().body("CUSTOMER_NOT_FOUND".to_string())
    }
}
#[delete("/customers/{customer_id}")]
pub async fn delete_customer(customer_id: web::Path<String>, req: HttpRequest) -> HttpResponse {
    let issuer_role = match req.extensions().get::<UserAuthentication>() {
        Some(issuer) => issuer.role.clone(),
        None => return HttpResponse::Unauthorized().body("UNAUTHORIZED".to_string()),
    };
    if issuer_role.is_empty()
        || !Role::validate(&issuer_role, &RolePermission::DeleteCustomer).await
    {
        return HttpResponse::Unauthorized().body("UNAUTHORIZED".to_string());
    }

    let customer_id = match customer_id.parse() {
        Ok(customer_id) => customer_id,
        _ => return HttpResponse::BadRequest().body("INVALID_ID".to_string()),
    };

    return match Customer::delete_customer(&customer_id).await {
        Ok(count) => HttpResponse::Ok().body(format!("Deleted {count} customer")),
        Err(error) => HttpResponse::InternalServerError().body(error),
    };
}
//+validasi get 1 per 1, update, delete,
