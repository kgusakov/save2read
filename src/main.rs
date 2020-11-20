mod storage;

use actix_web::{get, post, web, App, HttpResponse, HttpRequest, HttpServer, Responder};
use url::Url;
use std::sync::Mutex;
use askama::Template;
use storage::Storage;
use sqlx::sqlite::SqlitePoolOptions;

struct AppState {
    storage: Mutex<Storage>
}

#[derive(Template)] // this will generate the code...
#[template(path = "index.html")] // using the template in this path, relative
                                 // to the `templates` dir in the crate root
struct ListTemplate<'a> { // the name of the struct can be anything
    app_name: &'a str,
    links: &'a Vec<String>
}


#[get("/list/{user_id}")]
async fn list(web::Path(user_id): web::Path<String>, data: web::Data<AppState>) -> impl Responder {
    let d = data.storage.lock().unwrap();
    let links = d.pending_list(user_id)
        .await
        .iter()
        .map(|url| url.to_string())
        .collect::<Vec<String>>();

    HttpResponse::Ok().body(
        ListTemplate {
            app_name: "Save For Read",
            links: &links
        }.render().unwrap())
}

#[post("/add/{user_id}")]
async fn add(request: web::Bytes, web::Path(user_id): web::Path<String>,  data: web::Data<AppState>)  -> impl Responder {
    let mut d = data.storage.lock().unwrap();
    d.add(&user_id, Url::parse(&String::from_utf8(request.to_vec()).unwrap()).unwrap()).await;
    HttpResponse::Ok().body("Added!")
}

async fn manual_hello() -> impl Responder {
    HttpResponse::Ok().body("Hey there!")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {


    let db_pool = SqlitePoolOptions::new().connect("sqlite:/tmp/sqlite.db").await.unwrap();
    let app_state = web::Data::new(AppState {
        storage: Mutex::new(Storage::new(db_pool).await)
    });

    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .service(list)
            .service(add)
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}