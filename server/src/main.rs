use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder};
use tasko_shared::Task;
use tasko_shared::TaskState;
use tasko_shared::UpdateTaskStateRequest;
use tasko_shared::CreateTaskRequest;
use std::sync::{Arc, RwLock};
use uuid::Uuid;

type TaskList = Arc<RwLock<Vec<Task>>>;

fn init_tasks() -> TaskList {
    let tasks = vec![
        Task {
            id: Uuid::new_v4(),
            title: "Task 1".to_string(),
            description: "Description 1".to_string(),
            state: tasko_shared::TaskState::Todo,
        },
        Task {
            id: Uuid::new_v4(),
            title: "Task 2".to_string(),
            description: "Description 2".to_string(),
            state: tasko_shared::TaskState::InProgress,
        },
        Task {
            id: Uuid::new_v4(),
            title: "Task 3".to_string(),
            description: "Description 3".to_string(),
            state: tasko_shared::TaskState::Done,
        },
    ];

    Arc::new(RwLock::new(tasks))
}


#[get("/tasks")]
async fn get_tasks(task_list: web::Data<TaskList>) -> impl Responder {
    let tasks = task_list.read().unwrap();
    HttpResponse::Ok().json(tasks.clone())
}

#[post("/tasks")]
async fn create_task(
    task_list: web::Data<TaskList>,
    new_task_request: web::Json<CreateTaskRequest>,
) -> impl Responder {
    let mut tasks = task_list.write().unwrap();

    let new_task = Task {
        id: Uuid::new_v4(),
        title: new_task_request.title.clone(),
        description: new_task_request.description.clone(),
        state: TaskState::Todo,
    };

    tasks.push(new_task.clone());

    HttpResponse::Created().json(new_task)
}


#[post("/tasks/{id}/state")]
async fn update_task_state(
    task_list: web::Data<TaskList>,
    task_id: web::Path<Uuid>,
    update_request: web::Json<UpdateTaskStateRequest>,
) -> impl Responder {
    let mut tasks = task_list.write().unwrap();
    let id = task_id.into_inner();

    if let Some(task) = tasks.iter_mut().find(|t| t.id == id) {
        task.state = update_request.state.clone();
        HttpResponse::Ok().json(task.clone())
    } else {
        HttpResponse::NotFound().body("Task not found")
    }
}


#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let tasks = init_tasks();

    HttpServer::new(move || {
    App::new()
        .service(get_tasks)
        .service(update_task_state)
        .service(create_task)
        .app_data(web::Data::new(tasks.clone()))
    })
    .bind("127.0.0.1:3000")?
    .run()
    .await
}