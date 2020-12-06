use super::storage::Storage;
use actix_web::*;
use handlebars::Handlebars;
use serde::*;
use serde_json::*;
use std::sync::Arc;

const APP_NAME: &str = "Save to read";

pub struct AppState<'a> {
    pub storage: Arc<Storage>,
    pub hb: Arc<Handlebars<'a>>,
}

#[derive(Serialize, Deserialize, Debug)]
struct ListTemplate<'a> {
    // the name of the struct can be anything
    app_name: &'a str,
    links: Vec<(String, String, Option<String>)>,
    user_id: &'a str,
    page: &'a str,
}

#[get("/pending/{user_id}")]
pub async fn pending_list(
    web::Path(user_id): web::Path<String>,
    data: web::Data<AppState<'_>>,
) -> impl Responder {
    let d = &data.storage;
    let links = d
        .pending_list(&user_id)
        .await
        .unwrap()
        .into_iter()
        .map(|url| (url.0.to_string(), url.1.to_string(), url.2))
        .collect();
    let json = json!(ListTemplate {
        app_name: APP_NAME,
        links: links,
        user_id: &user_id,
        page: "pending"
    });
    let rendered = &data.hb.render("index", &json).unwrap();
    HttpResponse::Ok().body(rendered)
}

#[get("/archived/{user_id}")]
pub async fn archived_list(
    web::Path(user_id): web::Path<String>,
    data: web::Data<AppState<'_>>,
) -> impl Responder {
    let d = &data.storage;
    let links = d
        .archived_list(&user_id)
        .await
        .unwrap()
        .into_iter()
        .map(|url| (url.0.to_string(), url.1.to_string(), url.2))
        .collect();
    let json = json!(ListTemplate {
        app_name: APP_NAME,
        links: links,
        user_id: &user_id,
        page: "archived"
    });
    let rendered = &data.hb.render("index", &json).unwrap();
    HttpResponse::Ok().body(rendered)
}

#[delete("/archive/{link_id}")]
pub async fn archive(
    web::Path(link_id): web::Path<i64>,
    data: web::Data<AppState<'_>>,
) -> impl Responder {
    let d = &data.storage;
    d.archive(&link_id).await.unwrap();
    HttpResponse::Ok()
}

#[delete("/archived/delete/{link_id}")]
pub async fn delete_archived(
    web::Path(link_id): web::Path<i64>,
    data: web::Data<AppState<'_>>,
) -> impl Responder {
    let d = &data.storage;
    d.delete_archived(&link_id).await.unwrap();
    HttpResponse::Ok()
}

#[delete("/pending/delete/{link_id}")]
pub async fn delete_pending(
    web::Path(link_id): web::Path<i64>,
    data: web::Data<AppState<'_>>,
) -> impl Responder {
    let d = &data.storage;
    d.delete_pending(&link_id).await.unwrap();
    HttpResponse::Ok()
}
