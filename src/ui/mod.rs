pub mod gpu_panel;
pub mod jobs_panel;
pub mod log_panel;
pub mod new_job;

use crate::app::{App, Focus, Mode};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

pub fn draw(f: &mut Frame, app: &App) {
    let size = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(10), Constraint::Length(3)])
        .split(size);

    draw_header(f, chunks[0], app);
    draw_body(f, chunks[1], app);
    draw_footer(f, chunks[2], app);

    if matches!(app.mode, Mode::NewJob) {
        new_job::draw(f, size, app);
    }
}

fn draw_header(f: &mut Frame, area: Rect, app: &App) {
    let title = format!(
        " translate-dashboard │ endpoints={}  focus={:?}  ",
        app.cfg.api_endpoints.len(),
        app.focus
    );
    let p = Paragraph::new(title).style(
        Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD),
    );
    f.render_widget(p, area);
}

fn draw_body(f: &mut Frame, area: Rect, app: &App) {
    let main = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    let left = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(10), Constraint::Min(5)])
        .split(main[0]);

    gpu_panel::draw(f, left[0], app);
    jobs_panel::draw_history(f, left[1], app);

    let right = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(12), Constraint::Min(5)])
        .split(main[1]);

    jobs_panel::draw_active(f, right[0], app);
    log_panel::draw(f, right[1], app);
}

fn draw_footer(f: &mut Frame, area: Rect, app: &App) {
    let hint = match app.mode {
        Mode::Normal => " [n] new job │ [Tab] focus │ [q] quit │ [↑↓] scroll ",
        Mode::NewJob => " 새 작업: [Tab] 필드 이동 │ [Enter] 큐에 추가 │ [Esc] 취소 ",
    };
    let p = Paragraph::new(hint)
        .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded));
    f.render_widget(p, area);
    let _ = app.focus; // silence unused warning when Focus variants change
    let _ = Focus::Gpu;
}
