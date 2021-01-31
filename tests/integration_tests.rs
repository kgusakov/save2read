use actix_http::{cookie::Cookie, Error, Request};
use actix_service::Service;
use actix_web::body::MessageBody;
use actix_web::http;
use actix_web::{dev::ServiceResponse, test, App};
use handlebars::Handlebars;
use save2read::auth::*;
use save2read::routes::*;
use save2read::storage::*;
use save2read::*;
use sqlx::{sqlite::SqlitePoolOptions, Executor};
use std::fs::File;
use std::sync::Arc;
use tempdir::TempDir;

#[actix_rt::test]
async fn test_index_no_auth() {
    let mut app = app(init_state().await).await;
    let req = test::TestRequest::get().uri("/").to_request();
    let resp = test::call_service(&mut app, req).await;
    assert_eq!(http::StatusCode::FORBIDDEN, resp.status());
}

#[actix_rt::test]
async fn test_index_with_auth() {
    let state = init_state().await;
    create_article(&state.storage, 1, "http://link", "Title").await;
    create_article(&state.storage, 2, "http://link1", "Title1").await;
    let token_storage = state.token_storage.clone();
    let mut app = app(state).await;

    let authorized_req = test::TestRequest::get()
        .cookie(auth(&mut app, &1i64, &token_storage).await)
        .uri("/")
        .to_request();
    let result = test::call_service(&mut app, authorized_req).await;

    assert_eq!(http::StatusCode::OK, result.status());
    let body = String::from_utf8(test::read_body(result).await.to_vec()).unwrap();
    assert!(body.contains("http://link"));
    assert!(!body.contains("http://link1"));
}

#[actix_rt::test]
async fn test_archive_no_auth() {
    let state = init_state();
    let mut app = app(state.await).await;
    let req = test::TestRequest::get().uri("/archived").to_request();
    let resp = test::call_service(&mut app, req).await;
    assert_eq!(http::StatusCode::FORBIDDEN, resp.status());
}

#[actix_rt::test]
async fn test_archive_with_auth() {
    let state = init_state().await;
    let token_storage = state.token_storage.clone();
    create_article(&state.storage, 1, "http://linku1p", "Title").await;
    create_archived_article(&state.storage, 1, "http://linku1a", "Title").await;
    create_article(&state.storage, 2, "http://linku2p", "Title1").await;
    create_archived_article(&state.storage, 2, "http://linku2a", "Title1").await;
    let mut app = app(state).await;

    let authorized_req = test::TestRequest::get()
        .cookie(auth(&mut app, &1i64, &token_storage).await)
        .uri("/archived")
        .to_request();
    let result = test::call_service(&mut app, authorized_req).await;

    assert_eq!(http::StatusCode::OK, result.status());
    let body = String::from_utf8(test::read_body(result).await.to_vec()).unwrap();
    assert!(body.contains("http://linku1a"));
    assert!(!body.contains("http://linku1p"));
    assert!(!body.contains("http://linku2p"));
    assert!(!body.contains("http://linku2a"));
}

#[actix_rt::test]
async fn test_do_archive_no_auth() {
    let state = init_state();
    let mut app = app(state.await).await;
    let req = test::TestRequest::post().uri("/archive/1").to_request();
    let resp = test::call_service(&mut app, req).await;
    assert_eq!(http::StatusCode::FORBIDDEN, resp.status());
}

#[actix_rt::test]
async fn test_do_archive_correct_auth() {
    let state = init_state().await;
    let token_storage = state.token_storage.clone();
    let storage = state.storage.clone();
    create_article(&state.storage, 1, "http://linku1p", "Title").await;
    let mut app = app(state).await;

    let authorized_req = test::TestRequest::post()
        .cookie(auth(&mut app, &1i64, &token_storage).await)
        .uri("/archive/1")
        .to_request();
    let result = test::call_service(&mut app, authorized_req).await;

    assert_eq!(http::StatusCode::OK, result.status());
    assert_eq!(1, storage.archived_list(&1).await.unwrap().len());
    assert_eq!(0, storage.pending_list(&1).await.unwrap().len());
}

#[actix_rt::test]
async fn test_do_archive_incorrect_auth() {
    let state = init_state().await;
    let token_storage = state.token_storage.clone();
    let storage = state.storage.clone();
    create_article(&state.storage, 1, "http://linku1p", "Title").await;
    let mut app = app(state).await;

    let authorized_req = test::TestRequest::post()
        .cookie(auth(&mut app, &2i64, &token_storage).await)
        .uri("/archive/1")
        .to_request();
    let result = test::call_service(&mut app, authorized_req).await;

    assert_eq!(http::StatusCode::NOT_FOUND, result.status());
    assert_eq!(0, storage.archived_list(&1).await.unwrap().len());
    assert_eq!(1, storage.pending_list(&1).await.unwrap().len());
}

#[actix_rt::test]
async fn test_delete_pending_incorrect_auth() {
    let state = init_state().await;
    let token_storage = state.token_storage.clone();
    let storage = state.storage.clone();
    create_article(&state.storage, 1, "http://linku1p", "Title").await;
    let mut app = app(state).await;

    let authorized_req = test::TestRequest::delete()
        .cookie(auth(&mut app, &2i64, &token_storage).await)
        .uri("/pending/delete/1")
        .to_request();
    let result = test::call_service(&mut app, authorized_req).await;

    assert_eq!(http::StatusCode::OK, result.status());
    assert_eq!(1, storage.pending_list(&1).await.unwrap().len());
}

async fn auth<'a>(
    app: &mut impl Service<
        Request = Request,
        Response = ServiceResponse<impl MessageBody>,
        Error = Error,
    >,
    user_id: &i64,
    token_storage: &TokenStorage,
) -> Cookie<'a> {
    token_storage
        .push(user_id.clone(), "token".to_string())
        .await
        .unwrap();

    let resp = test::call_service(
        app,
        test::TestRequest::get().uri("/auth/token").to_request(),
    )
    .await;
    let cookies = resp
        .headers()
        .get_all(http::header::SET_COOKIE)
        .map(|v| v.to_str().unwrap().to_owned())
        .last()
        .unwrap();
    Cookie::parse_encoded(cookies).unwrap()
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
    let storage = Arc::new(Storage::init(db_pool.clone()).await.unwrap());
    let token_storage = Arc::new(TokenStorage::new(100));
    AppState {
        storage: storage.clone(),
        token_storage: token_storage.clone(),
        hb: handlebars_ref.clone(),
    }
}

async fn create_article(storage: &Storage, user_id: i64, url: &str, title: &str) -> i64 {
    let article = ArticleData {
        user_id,
        url: url::Url::parse(&url).unwrap(),
        title: Some(title.to_string()),
    };
    storage.add(article).await.unwrap()
}

async fn create_archived_article(storage: &Storage, user_id: i64, url: &str, title: &str) -> i64 {
    let pending_id = create_article(storage, user_id, url, title).await;
    storage
        .archive(&user_id, &pending_id)
        .await
        .unwrap()
        .unwrap()
}
