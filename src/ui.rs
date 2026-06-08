use crate::app::{App, Focus};
use ansi_to_tui::IntoText;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Text};
use ratatui::widgets::{Block, BorderType, List, ListItem, ListState, Paragraph, Wrap};

const ACCENT: Color = Color::Cyan;
const HIGHLIGHT_BG: Color = Color::Cyan;
const HIGHLIGHT_FG: Color = Color::Black;
const DIM: Color = Color::DarkGray;
const DESC_COLOR: Color = Color::Yellow;
const STATUS_BG: Color = Color::Blue;
const STATUS_FG: Color = Color::White;

fn border_color(focused: bool, default: Color) -> Color {
    if focused { ACCENT } else { default }
}

pub fn draw(frame: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(25), Constraint::Percentage(75)])
        .split(frame.area());

    // Split right side: description + output + status bar
    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(chunks[1]);

    // Store output viewport height for scroll calculations
    app.output_height = right_chunks[1].height.saturating_sub(2);

    let cmd_focused = app.focus == Focus::Commands;
    let output_focused = app.focus == Focus::Output;

    // Left pane: command list (names only)
    let items: Vec<ListItem> = app
        .commands
        .iter()
        .map(|cmd| {
            ListItem::new(Line::raw(format!(" {} ", cmd.name)))
                .style(Style::default().fg(Color::White))
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::bordered()
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(border_color(cmd_focused, ACCENT)))
                .title(format!(" Commands [{}] ", app.commands.len()))
                .title_style(Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
        )
        .highlight_style(Style::default().fg(HIGHLIGHT_FG).bg(HIGHLIGHT_BG));
    let mut list_state = ListState::default().with_selected(Some(app.selected));
    frame.render_stateful_widget(list, chunks[0], &mut list_state);

    // Description pane
    let desc_text = if app.commands.is_empty() {
        "No commands discovered".to_string()
    } else {
        let cmd = &app.commands[app.selected];
        match &cmd.description {
            Some(desc) => desc.clone(),
            None => format!("No description for `{}`", cmd.name),
        }
    };
    let description = Paragraph::new(desc_text)
        .style(Style::default().fg(DESC_COLOR))
        .block(
            Block::bordered()
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(DIM))
                .title(" Description ")
                .title_style(Style::default().fg(DESC_COLOR).add_modifier(Modifier::BOLD)),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(description, right_chunks[0]);

    // Output pane with ANSI color rendering
    let raw_output = app.output.join("\n");
    let output_text = if app.output.is_empty() {
        Text::raw("Press Enter to run selected command")
    } else {
        raw_output
            .as_bytes()
            .into_text()
            .unwrap_or_else(|_| Text::raw(raw_output.clone()))
    };

    let output_border = border_color(output_focused, DIM);
    let output_title_style = if output_focused {
        Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD | Modifier::REVERSED)
    } else {
        Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD)
    };

    let output = Paragraph::new(output_text)
        .block(
            Block::bordered()
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(output_border))
                .title(" Output ")
                .title_style(output_title_style),
        )
        .wrap(Wrap { trim: false })
        .scroll((app.output_scroll, 0));
    frame.render_widget(output, right_chunks[1]);

    // Status bar
    let state = if app.task.is_some() {
        "running".to_string()
    } else if let Some(code) = app.exit_code {
        format!("exit: {code}")
    } else {
        "idle".to_string()
    };
    let workspace_name = app
        .workspace
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    let status_style = if app.task.is_some() {
        Style::default().fg(Color::Black).bg(Color::Yellow)
    } else if app.exit_code == Some(0) {
        Style::default().fg(Color::Black).bg(Color::Green)
    } else if app.exit_code.is_some() {
        Style::default().fg(Color::White).bg(Color::Red)
    } else {
        Style::default().fg(STATUS_FG).bg(STATUS_BG)
    };

    let focus_hint = match app.focus {
        Focus::Commands => "commands",
        Focus::Output => "output (j/k scroll, g/G top/bottom, Esc back)",
    };

    let status_text = if let Some((msg, _)) = &app.flash_message {
        format!(" {msg}")
    } else {
        format!(
            " {workspace_name} | {state} | {} commands | [{focus_hint}]",
            app.commands.len()
        )
    };
    let status = Paragraph::new(status_text).style(status_style);
    frame.render_widget(status, right_chunks[2]);
}
