mod extractor;
mod routes;
mod storage;
mod telegram_api;

use actix_web::client::Client;
use actix_web::{web, App, HttpServer};
use extractor::*;
use handlebars::Handlebars;
use log::error;
use routes::*;
use sqlx::sqlite::SqlitePoolOptions;
use std::sync::Arc;
use storage::Storage;
use telegram_api::*;
use url::Url;
use anyhow::*;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();

    let mut handlebars = Handlebars::new();
    handlebars
        .register_templates_directory(".html", "./templates")
        .unwrap();
    let handlebars_ref = Arc::new(handlebars);

    let db_pool = SqlitePoolOptions::new()
        .connect("sqlite:/tmp/sqlite.db")
        .await
        .unwrap();
    let storage = Arc::new(Storage::init(db_pool).await.unwrap());

    let st = storage.clone();
    actix_rt::spawn(async move {
        update_loop(&st).await;
    });

    let st1 = storage.clone();
    let app_state = web::Data::new(AppState {
        storage: st1,
        hb: handlebars_ref.clone(),
    });

    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .service(pending_list)
            .service(archived_list)
            .service(archive)
            .service(delete_archived)
            .service(delete_pending)
    })
    .bind("192.168.1.83:8080")?
    .run()
    .await
}

async fn update_loop(storage: &Storage) {
    let client = Client::default();
    let api_token = std::env::var("BOT_TOKEN").expect("Provide telegram api token pls");
    let telegram_api = telegram_api::TelegramClient::new(api_token, &client);
    let mut update_id = -1;
    loop {
        match telegram_api.get_updates(update_id + 1).await {
            Ok(updates) => {
                for update in updates.result {
                    update_id = update.update_id;
                    if let Err(e) = process_upate(&update, storage, &telegram_api).await {
                        error!("{}", e);
                    }
                }
            }
            Err(err) => error!("{}", err),
        }
    }
}

async fn process_upate<'a>(update: &Update, storage: &Storage, telegram_api: &TelegramClient<'a>) -> Result<()> {
    if let Some(ref t) = update.message.text {
        let url = Url::parse(t)?;
        storage.add(update.message.chat.id, &url, extract(&url).await?)
            .await?;
        telegram_api
            .async_send_message(SendMessage {
                chat_id: format!("{}", update.message.chat.id),
                text: format!("{}", update.message.chat.id),
                reply_to_message_id: None,
            })
            .await?;
    }
    Ok(())
}