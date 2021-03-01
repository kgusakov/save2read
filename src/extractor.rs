use std::time::Duration;

use actix_web::http;
use actix_web::{client::Client, web::Bytes};
use anyhow::{anyhow, Result};
use encoding_rs::*;
use lazy_static::*;
use regex::*;
use scraper::{Html, Selector};
use std::borrow::Cow::*;

lazy_static! {
    static ref CHARSET_REGEXP: Regex = Regex::new("charset=([^;]+)").unwrap();
}

fn title(doc: &Html) -> Result<Option<String>> {
    let title_selector = Selector::parse("title")
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

fn charset(doc: &Html) -> Result<Option<&'static Encoding>> {
    let meta_selector = Selector::parse("meta")
        .map_err(|err| anyhow!("Can't parse selector for meta {:?}", err))?;
    Ok(doc
        .select(&meta_selector)
        .flat_map(|meta_tag| {
            meta_tag
                .value()
                .attr("charset")
                .and_then(|ch| Encoding::for_label(ch.as_bytes()))
                .or(meta_tag.value().attr("content").and_then(|cnt| {
                    CHARSET_REGEXP.captures(cnt).and_then(|caps| {
                        caps.get(1)
                            .and_then(|cap| Encoding::for_label(cap.as_str().as_bytes()))
                    })
                }))
                .into_iter()
        })
        .next())
}

pub async fn extract(url: &url::Url) -> Result<Option<String>> {
    let client = Client::builder().timeout(Duration::from_secs(60)).finish();
    if let Some(data) = ignore_redirects(&client, url.as_str(), 10).await? {
        let resp: Vec<u8> = data.to_vec();
        let html_str = String::from_utf8_lossy(&resp);
        let html = Html::parse_document(&html_str);
        if let Ok(Some(encoding)) = charset(&html) {
            let (decoded, _, _) = encoding.decode(&resp);
            let data = match decoded {
                Borrowed(b) => b.to_string(),
                Owned(o) => o,
            };
            Ok(title(&Html::parse_document(&data))?)
        } else {
            Ok((title(&html))?)
        }
    } else {
        Ok(None)
    }
}

async fn ignore_redirects(client: &Client, url: &str, max_redirect: i8) -> Result<Option<Bytes>> {
    let mut resp = client.get(url).send().await.map_err(|err| {
        anyhow!(
            "Can't send request for data extraction to url {} with error {:?}",
            url,
            err
        )
    })?;
    let mut redirects = max_redirect;
    while resp.status().is_redirection() && redirects > 0 {
        let location = resp.headers().get_all(http::header::LOCATION).last();
        if let Some(loc) = location {
            let str_loc = loc.to_str().map_err(|err| {
                anyhow!(
                    "Url {} can be parsed to string for receiving title, error: {:?}",
                    url,
                    err
                )
            })?;
            resp = client.get(str_loc).send().await.map_err(|err| {
                anyhow!(
                    "Can't send request for data extraction to url {} with error {:?}",
                    url,
                    err
                )
            })?;
        } else {
            break;
        }
        redirects = redirects - 1;
    }
    if resp.status().is_success() {
        Ok(Some(resp.body().limit(usize::MAX).await?))
    } else {
        Ok(None)
    }
}

#[actix_rt::test]
async fn test_charset() {
    let html = Html::parse_document(
        r#"
        <html>
        <meta charset="windows-1251"/>
        <meta charset="utf-8"/>
        </html>
        "#,
    );
    assert_eq!(encoding_rs::WINDOWS_1251, charset(&html).unwrap().unwrap());
}

#[actix_rt::test]
async fn test_charset_content() {
    let html = Html::parse_document(
        r#"
        <html>
        <meta name="viewport" content="width=device-width, initial-scale=1">
        <meta content="text/html; charset=koi8-r"/>
        <meta charset="utf-8"/>
        </html>
        "#,
    );
    assert_eq!(encoding_rs::KOI8_R, charset(&html).unwrap().unwrap());
}
