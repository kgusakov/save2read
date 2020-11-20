use url::Url;
use sqlx::{Pool, query};
use sqlx::sqlite::Sqlite;
use sqlx::Row;

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

    pub async fn new(pool: Pool<Sqlite>) -> Storage {
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
        ).execute(&pool).await.unwrap();
        Storage {
            pool
        }
    }

    pub async fn add(&mut self, id: &str, link: Url) {
        query("INSERT INTO pending_links(user_id, url) values(?, ?);")
            .bind(id)
            .bind(link.to_string())
            .execute(&self.pool)
            .await
            .unwrap();
    }

    pub async fn archive(&mut self, id: &str, link: Url) {
        query("BEGIN").execute(&self.pool).await.unwrap();
        query(&format!("DELETE * FROM {} where user_id = ? and url = ?", PENDING_LINKS_TABLE))
            .bind(id)
            .bind(link.to_string())
            .execute(&self.pool)
            .await
            .unwrap();
        query(&format!("INSERT INTO {}(user_id, url) values(?, ?);", ARCHIVED_LINKS_TABLE))
            .bind(id)
            .bind(link.to_string())
            .execute(&self.pool)
            .await
            .unwrap();
        query("COMMIT").execute(&self.pool).await.unwrap();
    }

    pub async fn pending_list(&self, id: String) -> Vec<Url> {
        let rows: Vec<sqlx::sqlite::SqliteRow> = query(&format!("SELECT url from {} where user_id = ?", PENDING_LINKS_TABLE))
            .bind(id)
            .fetch_all(&self.pool)
            .await
            .unwrap();
        rows.iter().map(|r| r.try_get::<String, &str>("url").unwrap())
            .map(|u| Url::parse(&u).unwrap())
            .collect()
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