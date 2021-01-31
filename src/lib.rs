pub mod auth;
pub mod extractor;
pub mod routes;
pub mod storage;
pub mod telegram_api;

use actix_session::*;
use actix_web::client::*;
use actix_web::*;
use anyhow::Result;
use auth::*;
use extractor::extract;
use log::error;
use routes::*;
use storage::*;
use telegram_api::*;
use url::Url;

pub fn configure_app(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/")
            .wrap(
                CookieSession::signed(&[0; 32]) // <- create cookie based session middleware
                    .secure(false),
            )
            .service(pending_list)
            .service(archived_list)
            .service(archive)
            .service(delete_archived)
            .service(delete_pending)
            .service(auth),
    );
}

pub async fn update_loop(storage: &Storage, token_storage: &TokenStorage, port: &str) {
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

pub async fn process_update<'a>(
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
                let title = extract(&url).await?;
                storage
                    .add(ArticleData {
                        user_id: update.message.chat.id,
                        url: url,
                        title: title
                    })
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
