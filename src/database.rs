use mongodb::{options::Credential, Client, Database};

static mut DB: Option<Database> = None;

pub async fn connect(uri: String) {
    let mut client = Client::with_uri_str(uri)
        .await
        .expect("Failed to connect to database");

    match (
        std::env::var("DATABASE_USERNAME"),
        std::env::var("DATABASE_PASSWORD"),
    ) {
        (Ok(username), Ok(password)) => {
            let credential = Credential::builder()
                .username(username)
                .password(password)
                .source("admin".to_string())
                .build();

            let options = mongodb::options::ClientOptions::builder()
                .credential(credential)
                .build();

            client = Client::with_options(options).expect("Failed to connect to database");
        }
        _ => (),
    };

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
