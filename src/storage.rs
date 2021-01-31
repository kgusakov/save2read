use anyhow::{Context, Result};
use sqlx::sqlite::Sqlite;
use sqlx::Row;
use sqlx::{query, Pool};
use url::Url;

pub static PENDING_LINKS_TABLE: &str = "pending_links";
pub static ARCHIVED_LINKS_TABLE: &str = "archived_links";

pub struct Storage {
    pool: Pool<Sqlite>,
}

pub struct Article {
    pub id: i64,
    pub data: ArticleData,
}

#[derive(Clone)]
pub struct ArticleData {
    pub user_id: i64,
    pub url: Url,
    pub title: Option<String>,
}

impl Storage {
    pub async fn init(pool: Pool<Sqlite>) -> Result<Storage> {
        sqlx::query(
            "
            CREATE TABLE IF NOT EXISTS pending_links (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id INTEGER NOT NULL,
                title TEXT NULL,
                url TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS archived_links (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id INTEGER NOT NULL,
                title TEXT NULL,
                url TEXT NOT NULL
            );
            ",
        )
        .execute(&pool)
        .await
        .with_context(|| format!("Can't init the database with needed tables"))?;
        Ok(Storage { pool })
    }

    pub async fn add(&self, article: ArticleData) -> Result<i64> {
        query("INSERT INTO pending_links(user_id, url, title) values(?, ?, ?);")
            .bind(article.user_id)
            .bind(article.url.to_string())
            .bind(article.title)
            .execute(&self.pool)
            .await
            .with_context(|| format!("Can't insert pending link to the storage"))
            .map(|done| done.last_insert_rowid())
    }

    pub async fn archive(&self, user_id: &i64, id: &i64) -> Result<Option<i64>> {
        match self.get_pending_url(id).await? {
            Some(article_data) => {
                if &article_data.user_id == user_id {
                    query("BEGIN")
                        .execute(&self.pool)
                        .await
                        .with_context(|| format!("Can't start db transaction for archive item"))?;

                    let archived_id = query(&format!(
                        "INSERT INTO {}(user_id, url, title) values(?, ?, ?);",
                        ARCHIVED_LINKS_TABLE
                    ))
                    .bind(article_data.user_id)
                    .bind(article_data.url.to_string())
                    .bind(article_data.title.clone())
                    .execute(&self.pool)
                    .await
                    .with_context(|| format!("Can't insert link to move it to archived"))
                    .map(|done| done.last_insert_rowid())?;

                    query(&format!("DELETE FROM {} where id = ?", PENDING_LINKS_TABLE))
                        .bind(id)
                        .execute(&self.pool)
                        .await
                        .with_context(|| {
                            format!("Can't delete the link from storage to move it to archived")
                        })?;
                    query("COMMIT").execute(&self.pool).await.with_context(|| {
                        format!(
                            "Can't commit transaction for archive the item {} for user {}",
                            article_data.url.to_string(),
                            article_data.user_id
                        )
                    })?;
                    Ok(Some(archived_id))
                } else {
                    Ok(None)
                }
            }
            None => Ok(None),
        }
    }

    pub async fn delete_archived(&self, user_id: &i64, id: &i64) -> Result<()> {
        query(&format!(
            "DELETE FROM {} where id = ? and user_id = ?",
            ARCHIVED_LINKS_TABLE
        ))
        .bind(id)
        .bind(user_id)
        .execute(&self.pool)
        .await
        .with_context(|| format!("Can't delete the link {} from archived", id))?;
        Ok(())
    }

    pub async fn delete_pending(&self, user_id: &i64, id: &i64) -> Result<()> {
        query(&format!(
            "DELETE FROM {} where id = ? and user_id = ?",
            PENDING_LINKS_TABLE
        ))
        .bind(id)
        .bind(user_id)
        .execute(&self.pool)
        .await
        .with_context(|| format!("Can't delete the link {} from pending", id))?;
        Ok(())
    }

    pub async fn get_pending_url(&self, id: &i64) -> Result<Option<ArticleData>> {
        self.get_url(id, PENDING_LINKS_TABLE).await
    }

    pub async fn get_archived_url(&self, id: &i64) -> Result<Option<ArticleData>> {
        self.get_url(id, ARCHIVED_LINKS_TABLE).await
    }

    async fn get_url(&self, id: &i64, table: &str) -> Result<Option<ArticleData>> {
        let rows: Vec<sqlx::sqlite::SqliteRow> = query(&format!(
            "SELECT user_id, url, title from {} where id = ?",
            table
        ))
        .bind(id)
        .fetch_all(&self.pool)
        .await
        .with_context(|| format!("Can't get pending list for user {}", id))?;

        match rows.first() {
            Some(u) => Ok(Some(ArticleData {
                user_id: u
                    .try_get("user_id")
                    .with_context(|| format!("Can't get field user_id from db"))?,
                url: u
                    .try_get("url")
                    .with_context(|| format!("No field url in the result"))
                    .and_then(|u| {
                        Url::parse(u).with_context(|| format!("Can't parse url received from db"))
                    })?,
                title: u.try_get::<Option<String>, &str>("title")?,
            })),
            None => Ok(None),
        }
    }

    pub async fn pending_list(&self, user_id: &i64) -> Result<Vec<(i64, Url, Option<String>)>> {
        let rows: Vec<sqlx::sqlite::SqliteRow> = query(&format!(
            "SELECT id, url, title from {} where user_id = ?",
            PENDING_LINKS_TABLE
        ))
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .with_context(|| format!("Can't get pending list for user {}", user_id))?;
        let mut result = vec![];
        for r in rows.iter() {
            result.push((
                r.try_get::<i64, &str>("id")?,
                Url::parse(&r.try_get::<String, &str>("url")?)?,
                r.try_get::<Option<String>, &str>("title")?,
            ));
        }
        Ok(result)
    }

    pub async fn archived_list(&self, user_id: &i64) -> Result<Vec<(i64, Url, Option<String>)>> {
        let rows: Vec<sqlx::sqlite::SqliteRow> = query(&format!(
            "SELECT id, url, title from {} where user_id = ?",
            ARCHIVED_LINKS_TABLE
        ))
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .with_context(|| format!("Can't get archived list for user {}", user_id))?;
        let mut result = vec![];
        for r in rows.iter() {
            result.push((
                r.try_get::<i64, &str>("id")?,
                Url::parse(&r.try_get::<String, &str>("url")?)?,
                r.try_get::<Option<String>, &str>("title")?,
            ));
        }
        Ok(result)
    }
}
