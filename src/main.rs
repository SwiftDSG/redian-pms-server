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

    HttpServer::new(move || {
        App::new()
            .service(routes::user::get_users)
            .service(routes::user::get_user)
    })
    .bind(("127.0.0.1", 8000))?
    .run()
    .await
}
