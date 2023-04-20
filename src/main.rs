use actix_web::{web, App, HttpServer};
use std::io;

mod routes;

async fn connect_to_database() -> Result<mongodb::Client, mongodb::error::Error> {
    let db_uri: String =
        std::env::var("MONGODB_URI").unwrap_or_else(|_| String::from("mongodb://localhost:27017"));
    let client: mongodb::Client = mongodb::Client::with_uri_str(db_uri).await?;
    Ok(client)
}

#[actix_web::main]
async fn main() -> io::Result<()> {
    let client: mongodb::Client = connect_to_database()
        .await
        .expect("Connecting to database failed");

    let db: mongodb::Database = client.database("overhous");

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(db.clone()))
            .service(routes::project::get_user)
    })
    .bind(("127.0.0.1", 8000))?
    .run()
    .await
}
