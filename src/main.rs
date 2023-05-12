use actix_web::{App, HttpServer};
use std::io;

mod database;
mod models;
mod routes;

#[actix_web::main]
async fn main() -> io::Result<()> {
    let port: u16 = 8000;
    let db_uri: String =
        std::env::var("MONGODB_URI").unwrap_or_else(|_| String::from("mongodb://localhost:27017"));

    database::connect(db_uri).await;
    models::user::load_keys();

    HttpServer::new(move || {
        App::new()
            .wrap(models::user::UserAuthenticationMiddlewareFactory)
            .service(routes::user::get_users)
            .service(routes::user::get_user)
            .service(routes::user::create_user)
            .service(routes::user::login)
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
    .bind(("127.0.0.1", port))?
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
