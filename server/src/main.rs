use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};
use tasko_shared::Task;

#[get("/tasks")]
async fn get_tasks() -> impl Responder {
    let tasks = vec![
        Task {
            title: "Task 1".to_string(),
            description: "Description 1".to_string(),
        },
    ];

    HttpResponse::Ok().json(tasks)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| App::new().service(get_tasks))
        .bind("127.0.0.1:3000")?
        .run()
        .await
}