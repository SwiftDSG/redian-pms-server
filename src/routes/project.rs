use actix_web::{delete, get, post, web, HttpMessage, HttpRequest, HttpResponse};
use mongodb::bson::oid::ObjectId;
use std::str::FromStr;

use crate::models::{
    project::{Project, ProjectQuery, ProjectRequest, ProjectStatus},
    user::UserAuthentication,
};

// #[get("/projects")]
// pub async fn get_projects() -> HttpResponse {
//     let query: ProjectQuery = ProjectQuery {
//         _id: None,
//         limit: None,
//     };

//     match Project::find_many(&query).await {
//         Ok(projects) => HttpResponse::Ok().json(projects),
//         Err(error) => HttpResponse::BadRequest().body(error),
//     }
// }
// #[get("/projects/{_id}")]
// pub async fn get_project(_id: web::Path<String>) -> HttpResponse {
//     let _id: String = _id.into_inner();
//     if let Ok(_id) = ObjectId::from_str(&_id) {
//         return match Project::find_by_id(&_id).await {
//             Ok(Some(project)) => HttpResponse::Ok().json(project),
//             Ok(None) => HttpResponse::NotFound().body("ROLE_NOT_FOUND".to_string()),
//             Err(error) => HttpResponse::InternalServerError().body(error),
//         };
//     } else {
//         HttpResponse::BadRequest().body("INVALID_ID".to_string())
//     }
// }
// #[delete("/projects/{_id}")]
// pub async fn delete_project(_id: web::Path<String>) -> HttpResponse {
//     let _id: String = _id.into_inner();
//     if let Ok(_id) = ObjectId::from_str(&_id) {
//         return match Project::delete_by_id(&_id).await {
//             Ok(count) => HttpResponse::Ok().body(format!("Deleted {count} project")),
//             Err(error) => HttpResponse::InternalServerError().body(error),
//         };
//     } else {
//         HttpResponse::BadRequest().body("INVALID_ID".to_string())
//     }
// }
#[post("/projects")]
pub async fn create_project(payload: web::Json<ProjectRequest>, req: HttpRequest) -> HttpResponse {
    let payload: ProjectRequest = payload.into_inner();

    let project: Project = Project {
        _id: None,
        customer_id: payload.customer_id,
        name: payload.name,
        code: payload.code,
        status: ProjectStatus::Pending,
        member: None,
    };

    println!("{:#?}", project);

    HttpResponse::Ok().body("Ok".to_string())
    // if let Some(issuer) = req.extensions().get::<UserAuthentication>() {
    //     if !Project::validate(&issuer.project, &"add_project".to_string()).await {
    //         let payload: ProjectRequest = payload.into_inner();

    //         let mut project: Project = Project {
    //             _id: None,
    //             name: payload.name,
    //             permission: Vec::<String>::new(),
    //         };

    //         for i in payload.permission.iter() {
    //             project.add_permission(i);
    //         }

    //         if project.permission.is_empty() {
    //             return HttpResponse::BadRequest()
    //                 .body("ROLE_MUST_HAVE_VALID_PERMISSION".to_string());
    //         }

    //         match project.save().await {
    //             Ok(id) => HttpResponse::Created().body(id.to_string()),
    //             Err(error) => HttpResponse::InternalServerError().body(error),
    //         }
    //     } else {
    //         HttpResponse::Unauthorized().body("UNAUTHORIZED".to_string())
    //     }
    // } else {
    //     HttpResponse::Unauthorized().body("UNAUTHORIZED".to_string())
    // }
}
