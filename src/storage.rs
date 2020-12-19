use anyhow::{Context, Result};
use sqlx::sqlite::Sqlite;
use sqlx::Row;
use sqlx::{query, Pool};
use url::Url;

static PENDING_LINKS_TABLE: &'static str = "pending_links";
static ARCHIVED_LINKS_TABLE: &'static str = "archived_links";

pub struct Storage {
    pool: Pool<Sqlite>,
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

    pub async fn add(&self, id: i64, link: &Url, title: Option<String>) -> Result<()> {
        query("INSERT INTO pending_links(user_id, url, title) values(?, ?, ?);")
            .bind(id)
            .bind(link.to_string())
            .bind(title)
            .execute(&self.pool)
            .await
            .with_context(|| format!("Can't insert pending link to the storage"))
            .map(|_| ())
    }

    pub async fn archive(&self, id: &i64) -> Result<()> {
        match self.get_url(id).await? {
            Some((user_id, url, title)) => {
                query("BEGIN")
                    .execute(&self.pool)
                    .await
                    .with_context(|| format!("Can't start db transaction for archive item"))?;
                query(&format!(
                    "INSERT INTO {}(user_id, url, title) values(?, ?, ?);",
                    ARCHIVED_LINKS_TABLE
                ))
                .bind(user_id)
                .bind(url.to_string())
                .bind(title)
                .execute(&self.pool)
                .await
                .with_context(|| format!("Can't insert link to move it to archived"))?;
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
                        url.to_string(),
                        user_id
                    )
                })?;
                Ok(())
            }
            None => Ok(()),
        }
    }

    pub async fn delete_archived(&self, id: &i64) -> Result<()> {
        query(&format!(
            "DELETE FROM {} where id = ?",
            ARCHIVED_LINKS_TABLE
        ))
        .bind(id)
        .execute(&self.pool)
        .await
        .with_context(|| format!("Can't delete the link {} from archived", id))?;
        Ok(())
    }

    pub async fn delete_pending(&self, id: &i64) -> Result<()> {
        query(&format!("DELETE FROM {} where id = ?", PENDING_LINKS_TABLE))
            .bind(id)
            .execute(&self.pool)
            .await
            .with_context(|| format!("Can't delete the link {} from pending", id))?;
        Ok(())
    }

    pub async fn get_url(&self, id: &i64) -> Result<Option<(i64, Url, Option<String>)>> {
        let rows: Vec<sqlx::sqlite::SqliteRow> = query(&format!(
            "SELECT user_id, url, title from {} where id = ?",
            PENDING_LINKS_TABLE
        ))
        .bind(id)
        .fetch_all(&self.pool)
        .await
        .with_context(|| format!("Can't get pending list for user {}", id))?;

        match rows.first() {
            Some(u) => Ok(Some((
                u.try_get("user_id")
                    .with_context(|| format!("Can't get field user_id from db"))?,
                u.try_get("url")
                    .with_context(|| format!("No field url in the result"))
                    .and_then(|u| {
                        Url::parse(u).with_context(|| format!("Can't parse url received from db"))
                    })?,
                u.try_get::<Option<String>, &str>("title")?,
            ))),
            None => Ok(None),
        }
    }

    pub async fn pending_list(&self, id: &i64) -> Result<Vec<(i64, Url, Option<String>)>> {
        let rows: Vec<sqlx::sqlite::SqliteRow> = query(&format!(
            "SELECT id, url, title from {} where user_id = ?",
            PENDING_LINKS_TABLE
        ))
        .bind(id)
        .fetch_all(&self.pool)
        .await
        .with_context(|| format!("Can't get pending list for user {}", id))?;
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

    pub async fn archived_list(&self, id: &i64) -> Result<Vec<(i64, Url, Option<String>)>> {
        let rows: Vec<sqlx::sqlite::SqliteRow> = query(&format!(
            "SELECT id, url, title from {} where user_id = ?",
            ARCHIVED_LINKS_TABLE
        ))
        .bind(id)
        .fetch_all(&self.pool)
        .await
        .with_context(|| format!("Can't get archived list for user {}", id))?;
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
