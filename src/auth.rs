use anyhow::{bail, Result};
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use std::{collections::HashMap, time::Instant};
use tokio::sync::Mutex;

const AUTH_TOKEN_TTL_SECS: u64 = 5 * 60;

pub struct TokenStorage {
    data: Mutex<HashMap<String, (i64, Instant)>>,
}

impl TokenStorage {
    pub fn new() -> TokenStorage {
        TokenStorage {
            data: Mutex::new(HashMap::new()),
        }
    }

    pub async fn push(&self, user_id: i64, token: String) -> Result<()> {
        self.clean().await?;
        let mut data = self.data.lock().await;
        if data.contains_key(&token) {
            bail!("Token clash is not a good idea: {}", token)
        } else {
            data.insert(token, (user_id, Instant::now()));
            Ok(())
        }
    }

    pub async fn pop(&self, token: &str) -> Result<Option<i64>> {
        self.clean().await?;
        let mut data = self.data.lock().await;
        Ok(data.remove(token).map(|d| d.0))
    }

    async fn clean(&self) -> Result<()> {
        let mut data = self.data.lock().await;
        let to_remove: Vec<String> = data
            .iter()
            .filter_map(|(token, (_, time))| {
                if time.elapsed().as_secs() > AUTH_TOKEN_TTL_SECS {
                    Some(token.clone())
                } else {
                    None
                }
            })
            .collect();
        to_remove.iter().for_each(|t| {
            data.remove(t);
        });
        Ok(())
    }
}

pub fn generate_token() -> String {
    thread_rng().sample_iter(&Alphanumeric).take(30).collect()
}
