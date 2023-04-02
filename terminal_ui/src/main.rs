use std::io;
use tasko_shared::{Task, TaskState};
use tui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Terminal,
};
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite::Message;
// use tokio::net::TcpStream;
use std::sync::mpsc;
use serde_json;
use url;
use futures_util::stream::StreamExt;
use futures::SinkExt;
use futures::stream::SplitSink;





enum AppState {
    ViewingTasks,
    CreatingTask { mode: EditMode, title: String, description: String },
}

enum EditMode {
    Title,
    Description,
}
type WSStream = SplitSink<WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>, Message>;

async fn connect_to_websocket(
    tx: mpsc::Sender<Task>,
) -> Result<WSStream, Box<dyn std::error::Error>> {
    let url = url::Url::parse("ws://localhost:3000/ws/")?;
    let (ws_stream, _) = tokio_tungstenite::connect_async(url).await?;
    let (ws_tx, mut ws_rx) = ws_stream.split();

    // Spawn a task to listen for messages from the server
    tokio::spawn(async move {
        while let Some(Ok(msg)) = ws_rx.next().await {
            if let Ok(task) = serde_json::from_str::<Task>(&msg.to_string()) {
                let _ = tx.send(task);
            }
        }
    });

    Ok::<WSStream, Box<dyn std::error::Error>>(ws_tx)
}

async fn create_task(title: &str, description: &str) -> Result<Task, reqwest::Error> {
    let url = "http://localhost:3000/tasks";
    let create_request = tasko_shared::CreateTaskRequest {
        title: title.to_string(),
        description: description.to_string(),
    };
    let response = reqwest::Client::new()
        .post(url)
        .json(&create_request)
        .send()
        .await?;
    let task: Task = response.json().await?;
    Ok(task)
}

async fn fetch_tasks() -> Result<Vec<Task>, reqwest::Error> {
    let url = "http://localhost:3000/tasks";
    let response = reqwest::get(url).await?;
    let tasks: Vec<Task> = response.json().await?;
    Ok(tasks)
}

fn task_widget(tasks: &[Task]) -> List {
    let task_list = tasks
        .iter()
        .map(|task| {
            ListItem::new(Spans::from(vec![
                Span::styled(task.title.clone(), Style::default().fg(Color::Yellow)),
                Span::raw(": "),
                Span::styled(task.description.clone(), Style::default().fg(Color::LightBlue)),
            ]))
        })
        .collect::<Vec<_>>();

    List::new(task_list).block(Block::default().borders(Borders::ALL))
}

fn render_widgets(f: &mut tui::Frame<CrosstermBackend<io::Stdout>>, area: Rect, tasks: &[Task]) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(33); 3])
        .split(area);

    let todo_tasks: Vec<Task> = tasks
        .iter()
        .cloned()
        .filter(|t| t.state == TaskState::Todo)
        .collect();
    let in_progress_tasks: Vec<Task> = tasks
        .iter()
        .cloned()
        .filter(|t| t.state == TaskState::InProgress)
        .collect();
    let done_tasks: Vec<Task> = tasks
        .iter()
        .cloned()
        .filter(|t| t.state == TaskState::Done)
        .collect();

    f.render_widget(task_widget(&todo_tasks), chunks[0]);
    f.render_widget(task_widget(&in_progress_tasks), chunks[1]);
    f.render_widget(task_widget(&done_tasks), chunks[2]);
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut tasks = tokio::runtime::Runtime::new().unwrap().block_on(fetch_tasks())?;

    let stdout = io::stdout();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    enable_raw_mode()?;

    terminal.clear()?;
    let mut app_state = AppState::ViewingTasks;

    // Create a channel for receiving new tasks from the WebSocket
    let (tx, rx) = mpsc::channel::<Task>();

     let ws_stream = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(connect_to_websocket(tx))?;

    loop {
        terminal.draw(|f| {
            let area = f.size();
            match &app_state {
                AppState::ViewingTasks => {
                    render_widgets(f, area, &tasks);
                }
                AppState::CreatingTask { mode, title, description } => {
                    let task_input = format!("Title: {}\nDescription: {}", title, description);
                    let paragraph = Paragraph::new(task_input)
                        .block(Block::default().borders(Borders::ALL).title("Create Task"))
                        .alignment(Alignment::Left)
                        .wrap(tui::widgets::Wrap { trim: true });
                    f.render_widget(paragraph, area);
                }
            }
        })?;

         // Check for new tasks from the WebSocket
        if let Ok(new_task) = rx.try_recv() {
            tasks.push(new_task);
        }

if let Event::Key(key_event) = event::read()? {
    match &mut app_state {
        AppState::ViewingTasks => {
            if key_event.code == KeyCode::Char('q') {
                break;
            } else if key_event.code == KeyCode::Char('n') {
                app_state = AppState::CreatingTask {
                    mode: EditMode::Title,
                    title: String::new(),
                    description: String::new(),
                };
            }
        }
        AppState::CreatingTask { mode, title, description } => {
            if key_event.code == KeyCode::Enter {
                let new_task = tokio::runtime::Runtime::new()
                    .unwrap()
                    .block_on(create_task(&title, &description))?;
                tasks.push(new_task);
                app_state = AppState::ViewingTasks;
            } else if key_event.code == KeyCode::Esc {
                app_state = AppState::ViewingTasks;
            } else if key_event.code == KeyCode::Tab {
                *mode = match mode {
                    EditMode::Title => EditMode::Description,
                    EditMode::Description => EditMode::Title,
                };
            } else {
                match (mode, key_event) {
                    (EditMode::Title, KeyEvent { code: KeyCode::Char(c), .. }) => {
                        if !title.is_empty() || c != ' ' {
                            title.push(c);
                        }
                    }
                    (EditMode::Title, KeyEvent { code: KeyCode::Backspace, .. }) => {
                        title.pop();
                    }
                    (EditMode::Description, KeyEvent { code: KeyCode::Char(c), .. }) => {
                        if !description.is_empty() || c != ' ' {
                            description.push(c);
                        }
                    }
                    (EditMode::Description, KeyEvent { code: KeyCode::Backspace, .. }) => {
                        description.pop();
                    }
                    _ => {}
                }
            }
        }
    }
}



    }

    disable_raw_mode()?;
    Ok(())
}
