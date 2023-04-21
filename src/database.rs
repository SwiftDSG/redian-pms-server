use mongodb::Database;

static mut DB: Option<Database> = None;

pub async fn connect(uri: String) {
    if let Ok(client) = mongodb::Client::with_uri_str(uri).await {
        unsafe {
            DB = Some(client.database("pms"));
        }
    } else {
        panic!("Failed to connect to database")
    }
}

pub fn get_db() -> Database {
    unsafe {
        if let Some(db) = &DB {
            return db.clone();
        } else {
            panic!("Database is not available yet!")
        }
    }
}
