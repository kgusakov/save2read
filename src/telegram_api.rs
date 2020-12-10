use std::str::from_utf8;

use actix_web::client::Client;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize)]
pub struct TelegramResponse<T> {
    pub ok: bool,
    pub result: T,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Update {
    pub update_id: i32,
    pub message: Message,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Message {
    pub message_id: i64,
    #[serde(default)]
    pub from: Option<User>,
    #[serde(default)]
    pub text: Option<String>,
    pub chat: Chat,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Chat {
    pub id: i64,
}

#[derive(Clone, Debug, Deserialize)]
pub struct User {
    pub id: i32,
    pub is_bot: bool,
    pub first_name: String,
    #[serde(default)]
    pub last_name: Option<String>,
    #[serde(default)]
    pub username: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct BotCommand<'a> {
    pub command: &'a str,
    pub description: &'a str,
}

#[derive(Debug, Serialize)]
pub enum ParseMode {
    MarkdownV2,
    Markdown,
    HTML,
}

#[derive(Debug, Serialize)]
pub struct SendMessage<'a> {
    pub chat_id: String,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parse_mode: Option<ParseMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply_to_message_id: Option<&'a i64>,
}

pub struct TelegramClient<'a> {
    token: String,
    async_http_client: &'a Client,
}

impl<'a> TelegramClient<'a> {
    const BASE_TELEGRAM_API_URL: &'static str = "https://api.telegram.org/bot";

    fn api_url(&self, method: &str) -> String {
        format!(
            "{}{}/{}",
            TelegramClient::BASE_TELEGRAM_API_URL,
            self.token,
            method
        )
    }

    pub fn new(token_value: String, async_http_client: &'a Client) -> TelegramClient<'a> {
        TelegramClient {
            token: token_value,
            async_http_client,
        }
    }

    pub async fn get_updates(&self, update_id: i32) -> Result<TelegramResponse<Vec<Update>>> {
        Ok(self
            .async_http_client
            .get(&self.api_url(&format!("getUpdates?offset={:?}", update_id)))
            .send()
            .await
            .map_err(|e| {
                anyhow::anyhow!(
                    "Failed to get updates from telegram server for offset {}",
                    e
                )
            })?
            .json()
            .await
            .with_context(|| {
                format!(
                    "Failed to get updates from telegram server for offset {}",
                    update_id
                )
            })?)
    }

    pub async fn set_command(&self, message: &Vec<BotCommand<'_>>) -> Result<()> {
        let json_body = serde_json::to_string(message).with_context(|| {
            format!(
                "Failed to serialize body to json for set command {:?}",
                message
            )
        });
        Ok(self
            .async_http_client
            .post(&self.api_url("setMyCommands"))
            .header("Content-Type", "application/json")
            .send_body(json_body?)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))
            .map(|_| ())?)
    }

    pub async fn async_send_message(&self, message: SendMessage<'_>) -> Result<()> {
        let json_body = serde_json::to_string(&message).with_context(|| {
            format!(
                "Failed to serialize body to json for sending message {:?}",
                message
            )
        });
        Ok(self
            .async_http_client
            .post(&self.api_url("sendMessage"))
            .header("Content-Type", "application/json")
            .send_body(json_body?)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))
            .map(|_| ())?)
    }
}
