use actix_web::{get, post, web, App, HttpResponse, HttpRequest, HttpServer, Responder};
use url::Url;
use std::sync::Mutex;


mod storage;

use  storage::Storage;

struct AppState {
    storage: Mutex<Storage>
}


#[get("/list/{user_id}")]
async fn list(web::Path(user_id): web::Path<String>, data: web::Data<AppState>) -> impl Responder {
    let d = data.storage.lock().unwrap();
    let body = d.pending_list(user_id)
        .await
        .iter()
        .map(|url| url.to_string())
        .collect::<Vec<String>>()
        .join("\n");

    HttpResponse::Ok().body(body)
}

#[post("/add/{user_id}")]
async fn add(request: web::Bytes, web::Path(user_id): web::Path<String>,  data: web::Data<AppState>)  -> impl Responder {
    let mut d = data.storage.lock().unwrap();
    println!("url: {}, user_id: {}", String::from_utf8(request.to_vec()).unwrap(), user_id);
    d.add(&user_id, Url::parse(&String::from_utf8(request.to_vec()).unwrap()).unwrap()).await;
    HttpResponse::Ok().body("Added!")
}

async fn manual_hello() -> impl Responder {
    HttpResponse::Ok().body("Hey there!")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {


    let app_state = web::Data::new(AppState {
        storage: Mutex::new(Storage::new())
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