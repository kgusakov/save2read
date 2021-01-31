use anyhow::{bail, Result};
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use std::{collections::HashMap, time::Instant};
use tokio::sync::Mutex;

pub struct TokenStorage {
    data: Mutex<HashMap<String, (i64, Instant)>>,
    token_ttl_secs: u64,
}

impl TokenStorage {
    pub fn new(token_ttl_secs: u64) -> TokenStorage {
        TokenStorage {
            data: Mutex::new(HashMap::new()),
            token_ttl_secs,
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
                if time.elapsed().as_secs() >= self.token_ttl_secs {
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

#[cfg(test)]
mod tests {
    use super::TokenStorage;

    #[actix_rt::test]
    async fn test_push_double_pop() {
        let token_storage = TokenStorage::new(10);
        token_storage.push(1, "token".to_string()).await.unwrap();
        assert_eq!(token_storage.pop("token").await.unwrap().unwrap(), 1);
        assert_eq!(token_storage.pop("token").await.ok(), Some(None));
    }

    #[actix_rt::test]
    async fn test_multiple_push_double_pop() {
        let token_storage = TokenStorage::new(10);
        token_storage.push(1, "token1".to_string()).await.unwrap();
        token_storage.push(2, "token2".to_string()).await.unwrap();

        assert_eq!(token_storage.pop("token1").await.unwrap().unwrap(), 1);
        assert_eq!(token_storage.pop("token1").await.ok(), Some(None));

        assert_eq!(token_storage.pop("token2").await.unwrap().unwrap(), 2);
        assert_eq!(token_storage.pop("token2").await.ok(), Some(None));
    }

    #[actix_rt::test]
    async fn test_ttl() {
        let token_storage = TokenStorage::new(0);
        token_storage.push(1, "token".to_string()).await.unwrap();
        assert_eq!(token_storage.pop("token").await.ok(), Some(None));
    }
}
