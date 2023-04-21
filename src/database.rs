use mongodb::{Client, Database};

static mut DB: Option<Database> = None;

pub async fn connect(uri: String) {
    let client = Client::with_uri_str(uri)
        .await
        .expect("Failed to connect to database");
    unsafe {
        DB = Some(client.database("pms"));
    }
}

pub fn get_db() -> Database {
    unsafe {
        let db = &DB;
        db.clone().expect("Database is not available yet!")
    }
}
