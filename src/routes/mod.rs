use actix_web::{get, web, HttpResponse};
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FileKind {
    ProjectDocumentation,
}

#[derive(Deserialize)]
pub struct FileQueryParams {
    pub kind: FileKind,
    pub name: String,
}

pub mod customer;
pub mod project;
pub mod role;
pub mod user;

#[get("/files")]
pub async fn get_file(query: web::Query<FileQueryParams>) -> HttpResponse {
    let path = match query.kind {
        FileKind::ProjectDocumentation => format!(
            "./files/reports/documentation/64a550fc690938011be0b324/{}",
            query.name
        ),
    };
    if let Ok(file) = fs::read(path) {
        HttpResponse::Ok().content_type("image/png").body(file)
    } else {
        HttpResponse::NotFound().body("CONTENT_NOT_FOUND")
    }
}
