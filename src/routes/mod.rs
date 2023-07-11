use actix_web::{get, web, HttpResponse};
use mime_guess::from_path;
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FileKind {
    ProjectDocumentation,
    CompanyImage,
    CustomerImage,
    UserImage,
}

#[derive(Deserialize)]
pub struct FileQueryParams {
    pub kind: FileKind,
    pub name: String,
}

pub mod company;
pub mod customer;
pub mod project;
pub mod role;
pub mod user;

#[get("/files")]
pub async fn get_file(query: web::Query<FileQueryParams>) -> HttpResponse {
    let path = match query.kind {
        FileKind::ProjectDocumentation => format!("./files/reports/documentation/{}", query.name),
        FileKind::CompanyImage => format!("./files/companies/{}", query.name),
        FileKind::CustomerImage => format!("./files/customers/{}", query.name),
        FileKind::UserImage => format!("./files/users/{}", query.name),
    };
    if let Ok(file) = fs::read(path.clone()) {
        let mime = from_path(path).first_or_octet_stream();
        HttpResponse::Ok().content_type(mime).body(file)
    } else {
        HttpResponse::NotFound().body("CONTENT_NOT_FOUND")
    }
}
