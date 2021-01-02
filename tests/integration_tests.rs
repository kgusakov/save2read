use actix_http::{Error, Request};
use actix_service::Service;
use actix_web::body::MessageBody;
use actix_web::http;
use actix_web::{
    dev::ServiceResponse,
    test, App,
};
use handlebars::Handlebars;
use save2read::auth::*;
use save2read::routes::*;
use save2read::storage::*;
use save2read::*;
use sqlx::sqlite::SqlitePoolOptions;
use std::fs::File;
use std::sync::Arc;
use tempdir::TempDir;

#[actix_rt::test]
async fn test_index_not_auth() {
    let mut app = app(init_state().await).await;
    let req = test::TestRequest::get().uri("/").to_request();
    let resp = test::call_service(&mut app, req).await;
    assert_eq!(http::StatusCode::FORBIDDEN, resp.status());
}

#[actix_rt::test]
async fn test_index_with_auth() {
    let state = init_state().await;
    state
        .token_storage
        .push(125504090, "token".to_string())
        .await
        .unwrap();
    state
        .storage
        .add(
            125504090,
            &url::Url::parse("http://link").unwrap(),
            Some("Title".to_string()),
        )
        .await
        .unwrap();
    state
        .storage
        .add(
            2,
            &url::Url::parse("http://link1").unwrap(),
            Some("Title1".to_string()),
        )
        .await
        .unwrap();
    let mut app = app(state).await;

    let resp = test::call_service(
        &mut app,
        test::TestRequest::get().uri("/auth/token").to_request(),
    )
    .await;
    let cookies = resp
        .headers()
        .get_all(http::header::SET_COOKIE)
        .map(|v| v.to_str().unwrap().to_owned())
        .last()
        .unwrap();

    let authorized_req = test::TestRequest::get()
        .cookie(http::Cookie::parse_encoded(cookies).unwrap())
        .uri("/")
        .to_request();
    let result = test::call_service(&mut app, authorized_req).await;

    assert_eq!(http::StatusCode::OK, result.status());
}

// TODO: This dirty way will lead to leaking one app instance + state per integration test
async fn app(
    state: AppState<'static>,
) -> impl Service<Request = Request, Response = ServiceResponse<impl MessageBody>, Error = Error> {
    test::init_service(App::new().data(state).configure(configure_app)).await
}

async fn init_state<'a>() -> AppState<'a> {
    let mut handlebars = Handlebars::new();
    handlebars
        .register_templates_directory(".html", "./templates")
        .unwrap();
    let handlebars_ref = Arc::new(handlebars);
    let tmp_dir = TempDir::new("sqlite").unwrap();
    let dir = tmp_dir.path().join("sqlite.db");

    File::create(dir.clone()).unwrap();

    let db_pool = SqlitePoolOptions::new()
        .connect(dir.to_str().unwrap())
        .await
        .unwrap();
    let storage = Arc::new(Storage::init(db_pool).await.unwrap());
    let token_storage = Arc::new(TokenStorage::new(100));
    AppState {
        storage: storage.clone(),
        token_storage: token_storage.clone(),
        hb: handlebars_ref.clone(),
    }
}
