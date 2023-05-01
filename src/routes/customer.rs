use actix_web::{get, post, web, HttpMessage, HttpRequest, HttpResponse};
use mongodb::bson::oid::ObjectId;
use regex::Regex;
use std::str::FromStr;

use crate::models::{
    role::Role,
    user::{User, UserAuthentication, UserCredential, UserQuery, UserRequest},
};