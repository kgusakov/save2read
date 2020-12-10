mod auth;
mod extractor;
mod routes;
mod storage;
mod telegram_api;

use actix_session::CookieSession;
use actix_web::client::Client;
use actix_web::{web, App, HttpServer};
use anyhow::*;
use auth::{generate_token, TokenStorage};
use extractor::*;
use handlebars::Handlebars;
use log::error;
use routes::*;
use sqlx::sqlite::SqlitePoolOptions;
use std::sync::Arc;
use storage::Storage;
use telegram_api::*;
use url::Url;

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
    let token_storage = Arc::new(TokenStorage::new());

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
            .wrap(
                CookieSession::signed(&[0; 32]) // <- create cookie based session middleware
                    .secure(false),
            )
            .app_data(app_state.clone())
            .service(pending_list)
            .service(archived_list)
            .service(archive)
            .service(delete_archived)
            .service(delete_pending)
            .service(auth)
    })
    .bind(format!("0.0.0.0:{}", port))?
    .run()
    .await
}

async fn update_loop(storage: &Storage, token_storage: &TokenStorage, port: &str) {
    let client = Client::default();
    let api_token = std::env::var("BOT_TOKEN").expect("Provide telegram api token pls");
    let host = std::env::var("SERVER_HOST").expect("Provide server host for generating urls");
    let base_url = format!("http://{}:{}", host, port);
    let telegram_api = telegram_api::TelegramClient::new(api_token, &client);
    let commands = vec![BotCommand {
        command: "auth",
        description: "get auth link for new devices",
    }];
    telegram_api.set_command(&commands).await.unwrap();
    let mut update_id = -1;
    loop {
        match telegram_api.get_updates(update_id + 1).await {
            Ok(updates) => {
                for update in updates.result {
                    update_id = update.update_id;
                    if let Err(e) =
                        process_update(&update, storage, token_storage, &telegram_api, &base_url)
                            .await
                    {
                        error!("{}", e);
                    }
                }
            }
            Err(err) => error!("{}", err),
        }
    }
}

async fn process_update<'a>(
    update: &Update,
    storage: &Storage,
    token_storage: &TokenStorage,
    telegram_api: &TelegramClient<'a>,
    base_url: &str,
) -> Result<()> {
    match update.message.text {
        Some(ref t) => {
            if t == "/auth" {
                let token = generate_token();

                token_storage
                    .push(update.message.chat.id, token.clone())
                    .await?;
                telegram_api
                    .async_send_message(SendMessage {
                        chat_id: format!("{}", update.message.chat.id),
                        text: format!(r#"{}/auth/{}"#, base_url, token),
                        reply_to_message_id: None,
                        parse_mode: Some(ParseMode::Markdown),
                    })
                    .await?;
            } else if let Ok(url) = Url::parse(&t) {
                storage
                    .add(update.message.chat.id, &url, extract(&url).await?)
                    .await?;
                telegram_api
                    .async_send_message(SendMessage {
                        chat_id: format!("{}", update.message.chat.id),
                        text: format!("{}", update.message.chat.id),
                        reply_to_message_id: None,
                        parse_mode: None,
                    })
                    .await?;
            }
        }
        None => (),
    };
    Ok(())
}
