use std::str::from_utf8;

use actix_web::client::Client;
use anyhow::{anyhow, Context, Result};
use scraper::{Html, Selector};

fn title(doc: &Html) -> Result<Option<String>> {
    let title_selector = Selector::parse("head > title")
        .map_err(|err| anyhow!("Can't parse selector for titile {:?}", err))?;
    match doc.select(&title_selector).next() {
        Some(el) => {
            let mut title = String::new();
            el.text().for_each(|t| title.push_str(t));
            if title.is_empty() {
                Ok(None)
            } else {
                Ok(Some(title))
            }
        }
        None => Ok(None),
    }
}

pub async fn extract(url: &url::Url) -> Result<Option<String>> {
    let client = Client::default();
    let resp: Vec<u8> = client
        .get(url.as_str())
        .send()
        .await
        .map_err(|err| {
            anyhow!(
                "Can't send request for data extraction to url {} with error {:?}",
                url,
                err
            )
        })?
        .body()
        .limit(usize::MAX)
        .await?
        .to_vec();
    let html_str =
        from_utf8(&resp).with_context(|| format!("Can't convert byte response to string"))?;
    let html = Html::parse_document(html_str);
    Ok((title(&html))?)
}

#[cfg(test)]
mod tests {
    use super::extract;

    #[actix_rt::test]
    async fn it_works() {
        let url = url::Url::parse("https://docs.rs/sqlx/0.3.5/sqlx/macro.query.html").unwrap();
        let res = extract(&url).await.unwrap().unwrap();
        println!("Title: {}", res);
    }
}
