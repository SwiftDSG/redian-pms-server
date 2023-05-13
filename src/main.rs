use actix_cors::Cors;
use actix_web::{http, App, HttpServer};
use std::{fs::read_to_string, io};

mod database;
mod models;
mod routes;

fn load_env() {
    if let Ok(env) = read_to_string(".env") {
        let lines: Vec<(&str, &str)> = env
            .lines()
            .map(|a| {
                let b: Vec<&str> = a.split("=").collect();
                (
                    b.get(0).expect("INVALID_ENVIRONMENT_VARIABLES").clone(),
                    b.get(1).expect("INVALID_ENVIRONMENT_VARIABLES").clone(),
                )
            })
            .collect();

        for (key, value) in lines {
            std::env::set_var(key.to_string(), value.to_string());
        }
    }

    if std::env::var("DATABASE_URI").is_err() {
        std::env::set_var("DATABASE_URI", "mongodb://localhost:27017".to_string());
    }
    if std::env::var("CLIENT_URL").is_err() {
        std::env::set_var("CLIENT_URL", "http://localhost:3000".to_string());
    }
    if std::env::var("BASE_URL").is_err() {
        std::env::set_var("BASE_URL", "http://localhost:8000".to_string());
    }
}

#[actix_web::main]
async fn main() -> io::Result<()> {
    load_env();

    database::connect(std::env::var("DATABASE_URI").unwrap()).await;
    models::user::load_keys();

    HttpServer::new(move || {
        let cors = Cors::permissive();
        // .allow_any_origin()
        // .allow_any_header()
        // .allow_any_method();
        // .allowed_origin("http://localhost:3000")
        // .allowed_methods(vec!["GET", "POST", "OPTIONS", "PATCH", "PUT"])
        // .allowed_headers(vec![http::header::AUTHORIZATION, http::header::ACCEPT])
        // .allowed_header(http::header::CONTENT_TYPE)
        // .max_age(3600);
        App::new()
            .wrap(models::user::UserAuthenticationMiddlewareFactory)
            .wrap(cors)
            .service(routes::user::get_users)
            .service(routes::user::get_user)
            .service(routes::user::create_user)
            .service(routes::user::login)
            .service(routes::user::refresh)
            .service(routes::role::create_role)
            .service(routes::customer::get_customers)
            .service(routes::customer::get_customer)
            .service(routes::customer::update_customer)
            .service(routes::customer::create_customer)
            .service(routes::project::get_project)
            .service(routes::customer::delete_customer)
            .service(routes::project::create_project)
            .service(routes::project::create_project_role)
            .service(routes::project::create_project_task)
            .service(routes::project::create_project_task_sub)
            .service(routes::project::create_project_report)
            .service(routes::project::update_project_task)
            .service(routes::project::update_project_task_period)
            .service(routes::project::update_project_task_status)
            .service(routes::project::update_project_report)
            .service(routes::project::add_project_member)
            .service(routes::project::add_project_area)
    })
    .bind(("127.0.0.1", 8000))?
    .workers(8)
    .run()
    .await

    // Alternative
    // HttpServer::new(move || {
    //     App::new()
    //         .route("/users", web::get().to(handlers::get_users))
    //         .route("/users/{id}", web::get().to(handlers::get_user_by_id))
    //         .route("/users", web::post().to(handlers::add_user))
    //         .route("/users/{id}", web::delete().to(handlers::delete_user))
}
