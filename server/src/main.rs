use actix_web::{get, post, web, App, HttpResponse, HttpServer, HttpRequest, Responder, middleware::Logger};
use actix_web_actors::ws;
use tasko_shared::{CreateTaskRequest, Task, TaskState, UpdateTaskStateRequest};
use std::sync::{Arc, RwLock};
use uuid::Uuid;
use actix::Actor;
use actix::StreamHandler;
use actix::Addr;
use actix::Handler;
use actix_web_actors::ws::{Message, ProtocolError};
use actix_web_actors::ws::WebsocketContext;
use std::collections::HashMap;

use actix::Message as ActixMessage;

pub struct TaskMessage(pub Task);

impl ActixMessage for TaskMessage {
    type Result = ();
}

#[derive(Clone)]
struct WebSocket {
    task_list: TaskList,
    clients: Arc<RwLock<HashMap<Uuid, Addr<WebSocket>>>>,
}


impl WebSocket {
    fn new(task_list: TaskList, clients: Arc<RwLock<HashMap<Uuid, Addr<WebSocket>>>>) -> Self {
        WebSocket {
            task_list,
            clients,
        }
    }
}

impl Actor for WebSocket {
    type Context = ws::WebsocketContext<Self>;
}

impl Handler<TaskMessage> for WebSocket {
    type Result = ();

    fn handle(&mut self, task_message: TaskMessage, ctx: &mut Self::Context) -> Self::Result {
        let task = task_message.0;
        for client in self.clients.read().unwrap().values() {
            client.do_send(TaskMessage(task.clone()));
        }
    }
}


impl StreamHandler<Result<Message, ProtocolError>> for WebSocket {
    fn handle(&mut self, msg: Result<Message, ProtocolError>, ctx: &mut WebsocketContext<Self>) {
        match msg {
            Ok(Message::Ping(msg)) => ctx.pong(&msg),
            Ok(Message::Pong(_)) => (),
            _ => (),
        }
    }
}


async fn start_websocket(
    req: HttpRequest,
    stream: web::Payload,
    app_state: web::Data<AppState>,
) -> Result<HttpResponse, actix_web::Error> {
    let tasks = app_state.tasks.clone();
    let clients = app_state.clients.read().unwrap().clone();

    let client_id = Uuid::new_v4();
    let websocket_actor = WebSocket::new(tasks, clients.read().unwrap().clone());

    clients.write().unwrap().insert(client_id, websocket_actor.clone().start());

    ws::start(websocket_actor, &req, stream)
}


type TaskList = Arc<RwLock<Vec<Task>>>;

struct AppState {
    tasks: TaskList,
    clients: RwLock<HashMap<Uuid, Addr<WebSocket>>>,
}



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

fn init_clients() -> RwLock<HashMap<Uuid, Addr<WebSocket>>> {
    RwLock::new(HashMap::new())
}


#[get("/tasks")]
async fn get_tasks(app_state: web::Data<AppState>) -> impl Responder {
    let tasks = app_state.tasks.read().unwrap();
    HttpResponse::Ok().json(tasks.clone())
}

#[post("/tasks")]
async fn create_task(
    data: web::Data<AppState>,
    new_task_request: web::Json<CreateTaskRequest>,
) -> impl Responder {
    let tasks = data.tasks.clone();
    let clients = data.clients.read().unwrap().clone();
    let mut tasks = tasks.write().unwrap();

    let new_task = Task {
        id: Uuid::new_v4(),
        title: new_task_request.title.clone(),
        description: new_task_request.description.clone(),
        state: TaskState::Todo,
    };

    tasks.push(new_task.clone());

    let clients = clients.read().unwrap();
    for client in clients.values() {
        client.do_send(TaskMessage(new_task.clone()));
    }

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
    std::env::set_var("RUST_LOG", "debug");
    env_logger::init();

    let tasks = init_tasks();
    let clients = init_clients(); 

    HttpServer::new(move || {
    App::new()
        .wrap(Logger::default())
        .service(get_tasks)
        .service(update_task_state)
        .service(create_task)
        .service(web::resource("/ws/").route(web::get().to(start_websocket)))
        .app_data(web::Data::new(AppState {
            tasks: tasks.clone(),
            clients: clients.read().unwrap().clone(),
        }))
    })
    .bind("127.0.0.1:3000")?
    .run()
    .await
}