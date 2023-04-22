use actix_web::{App, HttpServer};
use std::io;

mod database;
mod models;
mod routes;

#[actix_web::main]
async fn main() -> io::Result<()> {
    let db_uri: String =
        std::env::var("MONGODB_URI").unwrap_or_else(|_| String::from("mongodb://localhost:27017"));

    database::connect(db_uri).await;
    models::user::load_keys();

    HttpServer::new(move || {
        App::new()
            .service(routes::user::get_users)
            .service(routes::user::get_user)
            .service(routes::user::create_user)
            .service(routes::user::login)
    })
    .bind(("127.0.0.1", 8000))?
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
