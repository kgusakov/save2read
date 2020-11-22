use url::Url;
use sqlx::{Pool, query};
use sqlx::sqlite::Sqlite;
use sqlx::Row;
use anyhow::{Context, Result};

static PENDING_LINKS_TABLE: &'static str = "pending_links";
static ARCHIVED_LINKS_TABLE: &'static str = "archived_links";

pub struct Storage {
    pool: Pool<Sqlite>
}

struct Links {
    pending: Vec<Url>,
    archived: Vec<Url>,
}

impl Storage {

    pub async fn init(pool: Pool<Sqlite>) -> Result<Storage> {
        sqlx::query(
            "
            CREATE TABLE IF NOT EXISTS pending_links (
                user_id INTEGER NOT NULL,
                url TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS archived_links (
                user_id INTEGER NOT NULL,
                url TEXT NOT NULL
            );
            "
        ).execute(&pool)
            .await
            .with_context(|| format!("Can't init the database with needed tables"))?;
        Ok(Storage { pool })
    }

    pub async fn add(&self, id: &str, link: Url) -> Result<()> {
        query("INSERT INTO pending_links(user_id, url) values(?, ?);")
            .bind(id)
            .bind(link.to_string())
            .execute(&self.pool)
            .await
            .with_context(|| format!("Can't insert pending link to the storage"))
            .map(|_| ())
    }

    pub async fn archive(&self, id: &str, link: Url) -> Result<()> {
        query("BEGIN").execute(&self.pool)
            .await
            .with_context(|| format!("Can't start db transaction for archive item"))?;
        query(&format!("DELETE * FROM {} where user_id = ? and url = ?", PENDING_LINKS_TABLE))
            .bind(id)
            .bind(link.to_string())
            .execute(&self.pool)
            .await
            .with_context(|| format!("Can't delete the link from storage to move it to archived"))?;
        query(&format!("INSERT INTO {}(user_id, url) values(?, ?);", ARCHIVED_LINKS_TABLE))
            .bind(id)
            .bind(link.to_string())
            .execute(&self.pool)
            .await
            .with_context(|| format!("Can't insert link to move it to archived"))?;
        query("COMMIT").execute(&self.pool).await
            .with_context(|| format!("Can't commit transaction for archive the item {} for user {}", id, link))?;
        Ok(())
    }

    pub async fn pending_list(&self, id: &str) -> Result<Vec<Url>> {
        let rows: Vec<sqlx::sqlite::SqliteRow> = query(&format!("SELECT url from {} where user_id = ?", PENDING_LINKS_TABLE))
            .bind(id)
            .fetch_all(&self.pool)
            .await
            .with_context(|| format!("Can't get pending list for user {}", id))?;
        Ok(rows.iter().map(|r| r.try_get::<String, &str>("url").ok())
            .filter_map(|u| u.and_then(|ur| Url::parse(&ur).ok()))
            .collect())
    }

    pub async fn archived_list(&self) -> Vec<Url> {
        unimplemented!();
    }
}

#[cfg(test)]
mod tests {
    use sqlx::sqlite::*;
    use futures::executor::block_on;
    use futures::TryStreamExt;
    use sqlx::Row;
    
    #[actix_rt::test]
    async fn test () {
        let db_pool = SqlitePoolOptions::new().connect("sqlite:/tmp/sqlite.db").await.unwrap();
        let mut rows = sqlx::query(
            "
            CREATE TABLE IF NOT EXISTS pending_links (
                user_id INTEGER NOT NULL,
                url TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS archived_links (
                user_id INTEGER NOT NULL,
                url TEXT NOT NULL
            );
            "
        ).execute(&db_pool).await.unwrap();

        sqlx::query("
            INSERT INTO pending_links(user_id, url) VALUES(1, \"http://google.com\");
        ").execute(&db_pool).await.unwrap();
        
        let mut rows = sqlx::query("
            SELECT * FROM pending_links;
        ").fetch(&db_pool);

        while let Some(row) = rows.try_next().await.unwrap() {
            let user_id: i64 = row.try_get("user_id").unwrap(); 
            let url: String = row.try_get("url").unwrap();
            // map the row into a user-defined domain type
            println!("{} {}", user_id, url);
        }
    }
}