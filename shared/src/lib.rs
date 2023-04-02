use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub enum TaskState {
    Todo,
    InProgress,
    Done,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Task {
    pub id: Uuid,
    pub title: String,
    pub description: String,
    pub state: TaskState,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UpdateTaskStateRequest {
    pub state: TaskState,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateTaskRequest {
    pub title: String,
    pub description: String,
}

