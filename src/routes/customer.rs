use actix_web::{get, post, web, HttpMessage, HttpRequest, HttpResponse};
use mongodb::bson::oid::ObjectId;
use regex::Regex;
use std::str::FromStr;

use crate::models::customer::{Customer, CustomerQuery, CustomerRequest};

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
    match customer.save().await {
        Ok(id) => HttpResponse::Created().body(id.to_string()),
        Err(error) => HttpResponse::InternalServerError().body(error),
    }
}
//+validasi get 1 per 1, update, delete,
