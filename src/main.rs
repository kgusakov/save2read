use actix_web::{web, App, HttpServer};
use handlebars::Handlebars;
use routes::*;
use save2read::auth::TokenStorage;
use save2read::*;
use sqlx::sqlite::SqlitePoolOptions;
use std::sync::Arc;
use storage::Storage;

const TOKEN_TTL: u64 = 120;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();

    let mut handlebars = Handlebars::new();
    handlebars
        .register_templates_directory(".html", "./templates")
        .unwrap();
    let handlebars_ref = Arc::new(handlebars);
    let db_path = std::env::var("DB_PATH").expect("Provide database path");
    let db_pool = SqlitePoolOptions::new()
        .connect(&format!("sqlite:{}", &db_path))
        .await
        .unwrap();
    let storage = Arc::new(Storage::init(db_pool).await.unwrap());
    let token_storage = Arc::new(TokenStorage::new(TOKEN_TTL));

    let st = storage.clone();
    let tt = token_storage.clone();
    let port = std::env::var("SERVER_PORT").expect("Provide server port");
    let p = port.clone();
    actix_rt::spawn(async move {
        update_loop(&st, &tt, &p).await;
    });

    let st1 = storage.clone();
    let app_state = web::Data::new(AppState {
        storage: st1,
        hb: handlebars_ref.clone(),
        token_storage: token_storage,
    });

    HttpServer::new(move || {
        App::new()
            .configure(configure_app)
            .app_data(app_state.clone())
    })
    .bind(format!("0.0.0.0:{}", port))?
    .run()
    .await
}
