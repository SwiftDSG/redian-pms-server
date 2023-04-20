use actix_web::{get, web, HttpResponse};
use mongodb;

#[get("/{_id}")]
pub async fn get_user(
    client: web::Data<mongodb::Database>,
    _id: web::Path<String>,
) -> HttpResponse {
    let params: String = _id.into_inner();
    println!("{:?}", params);
    // let username = username.into_inner();
    // let collection: Collection<User> = client.database(DB_NAME).collection(COLL_NAME);
    // match collection
    //     .find_one(doc! { "username": &username }, None)
    //     .await
    // {
    //     Ok(Some(user)) => HttpResponse::Ok().json(user),
    //     Ok(None) => {
    //         HttpResponse::NotFound().body(format!("No user found with username {username}"))
    //     }
    //     Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
    // }
    HttpResponse::Ok().json(params)
}
