use std::io;
use tasko_shared::Task;
use tui:: {
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};

async fn fetch_tasks() -> Result<Vec<Task>, reqwest::Error> {
    let url = "http://localhost:3000/tasks";
    let response = reqwest::get(url).await?;
    let tasks: Vec<Task> =  response.json().await?;
    Ok(tasks)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let tasks = tokio::runtime::Runtime::new().unwrap().block_on(fetch_tasks())?;

    let stdout = io::stdout();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    terminal.clear()?;

    let task_list = tasks.iter().map(|task| {
        Spans::from(vec![
            Span::styled(task.title.clone(), Style::default().fg(Color::Yellow)),
            Span::raw(": "),
            Span::styled(task.description.clone(), Style::default().fg(Color::LightBlue)),
        ])
    }).collect::<Vec<_>>();

    let task_paragraph = Paragraph::new(task_list)
        .block(Block::default().borders(Borders::ALL).title("tasko"))
        .alignment(Alignment::Left);

    terminal.draw(|f| {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([Constraint::Percentage(100)].as_ref())
            .split(f.size());

        f.render_widget(task_paragraph, chunks[0]);
    })?;

    Ok(())
}