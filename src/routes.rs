use crate::auth::TokenStorage;

use super::storage::Storage;
use actix_session::Session;
use actix_web::*;
use handlebars::Handlebars;
use serde::*;
use serde_json::*;
use std::sync::Arc;

const APP_NAME: &str = "Save to read";

#[derive(Serialize, Deserialize, Debug)]
struct UserSession {
    user_id: i64,
}

pub struct AppState<'a> {
    pub storage: Arc<Storage>,
    pub token_storage: Arc<TokenStorage>,
    pub hb: Arc<Handlebars<'a>>,
}

#[derive(Serialize, Deserialize, Debug)]
struct ListTemplate<'a> {
    app_name: &'a str,
    links: Vec<(String, String, String)>,
    user_id: i64,
    page: &'a str,
}

#[get("")]
pub async fn pending_list(
    data: web::Data<AppState<'_>>,
    session: Session,
) -> std::result::Result<HttpResponse, actix_web::error::Error> {
    if let Some(user_id) = session.get::<UserSession>("user")? {
        let d = &data.storage;
        let links = d
            .pending_list(&user_id.user_id)
            .await
            .map_err(actix_web::error::ErrorInternalServerError)?
            .into_iter()
            .map(|url| (url.0.to_string(), url.1.to_string(), url.2.unwrap_or(url.1.to_string())))
            .collect();
        let json = json!(ListTemplate {
            app_name: APP_NAME,
            links,
            user_id: user_id.user_id,
            page: "pending"
        });
        let rendered = &data
            .hb
            .render("index", &json)
            .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;
        Ok(HttpResponse::Ok().body(rendered))
    } else {
        Ok(HttpResponse::Forbidden().finish())
    }
}

#[get("/archived")]
pub async fn archived_list(
    data: web::Data<AppState<'_>>,
    session: Session,
) -> std::result::Result<HttpResponse, actix_web::error::Error> {
    if let Some(user_id) = session.get::<UserSession>("user")? {
        let d = &data.storage;
        let links = d
            .archived_list(&user_id.user_id)
            .await
            .map_err(|e| actix_web::error::ErrorInternalServerError(e))?
            .into_iter()
            .map(|url| (url.0.to_string(), url.1.to_string(), url.2.unwrap_or(url.1.to_string())))
            .collect();
        let json = json!(ListTemplate {
            app_name: APP_NAME,
            links,
            user_id: user_id.user_id,
            page: "archived"
        });
        let rendered = &data
            .hb
            .render("index", &json)
            .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;
        Ok(HttpResponse::Ok().body(rendered))
    } else {
        Ok(HttpResponse::Forbidden().finish())
    }
}

#[post("/archive/{link_id}")]
pub async fn archive(
    web::Path(link_id): web::Path<i64>,
    data: web::Data<AppState<'_>>,
    session: Session,
) -> std::result::Result<HttpResponse, actix_web::error::Error> {
    if let Some(user) = session.get::<UserSession>("user")? {
        let d = &data.storage;
        let pending_result = d
            .get_pending_url(&link_id)
            .await
            .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;
        match pending_result {
            Some(link_info) if link_info.user_id == user.user_id => {
                d.archive(&user.user_id, &link_id)
                    .await
                    .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;
                Ok(HttpResponse::Ok().finish())
            }
            _ => Ok(HttpResponse::NotFound().finish()),
        }
    } else {
        Ok(HttpResponse::Forbidden().finish())
    }
}

#[delete("/archived/delete/{link_id}")]
pub async fn delete_archived(
    web::Path(link_id): web::Path<i64>,
    data: web::Data<AppState<'_>>,
    session: Session,
) -> std::result::Result<HttpResponse, actix_web::error::Error> {
    if let Some(user) = session.get::<UserSession>("user")? {
        let d = &data.storage;
        d.delete_archived(&user.user_id, &link_id)
            .await
            .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;
        Ok(HttpResponse::Ok().finish())
    } else {
        Ok(HttpResponse::Forbidden().finish())
    }
}

#[delete("/pending/delete/{link_id}")]
pub async fn delete_pending(
    web::Path(link_id): web::Path<i64>,
    data: web::Data<AppState<'_>>,
    session: Session,
) -> std::result::Result<HttpResponse, actix_web::error::Error> {
    if let Some(user) = session.get::<UserSession>("user")? {
        let d = &data.storage;
        d.delete_pending(&user.user_id, &link_id)
            .await
            .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;
        Ok(HttpResponse::Ok().finish())
    } else {
        Ok(HttpResponse::Forbidden().finish())
    }
}

#[get("/auth/{token}")]
pub async fn auth(
    web::Path(token): web::Path<String>,
    data: web::Data<AppState<'_>>,
    session: Session,
) -> std::result::Result<HttpResponse, actix_web::error::Error> {
    let token_storage = &data.token_storage.clone();
    if let Some(t) = token_storage
        .pop(&token)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?
    {
        session.set("user", UserSession { user_id: t.clone() })?;
    }

    Ok(HttpResponse::Found()
        .header(http::header::LOCATION, "/")
        .finish())
}
