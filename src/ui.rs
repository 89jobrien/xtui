use crate::app::App;
use ansi_to_tui::IntoText;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Text};
use ratatui::widgets::{Block, List, ListItem, Paragraph, Wrap};

pub fn draw(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(frame.area());

    // Split right side for output + status bar
    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(chunks[1]);

    // Left pane: command list
    let items: Vec<ListItem> = app
        .commands
        .iter()
        .enumerate()
        .map(|(i, cmd)| {
            let prefix = if i == app.selected { "> " } else { "  " };
            let label = match &cmd.description {
                Some(desc) => format!("{prefix}{:<16} {desc}", cmd.name),
                None => format!("{prefix}{}", cmd.name),
            };
            let style = if i == app.selected {
                Style::default().add_modifier(Modifier::REVERSED)
            } else {
                Style::default()
            };
            ListItem::new(Line::raw(label)).style(style)
        })
        .collect();

    let list = List::new(items)
        .block(Block::bordered().title(format!(" Commands [{}] ", app.commands.len())));
    frame.render_widget(list, chunks[0]);

    // Right pane: output with ANSI color rendering
    let raw_output = app.output.join("\n");
    let output_text = if app.output.is_empty() {
        Text::raw("Press Enter to run selected command")
    } else {
        raw_output
            .as_bytes()
            .into_text()
            .unwrap_or_else(|_| Text::raw(raw_output.clone()))
    };

    let output = Paragraph::new(output_text)
        .block(Block::bordered().title(" Output "))
        .wrap(Wrap { trim: false })
        .scroll((
            app.output
                .len()
                .saturating_sub(right_chunks[0].height.saturating_sub(2) as usize)
                as u16,
            0,
        ));
    frame.render_widget(output, right_chunks[0]);

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
    let status_text = if let Some((msg, _)) = &app.flash_message {
        format!(" {msg}")
    } else {
        format!(
            " {workspace_name} | {state} | {} commands",
            app.commands.len()
        )
    };
    let status =
        Paragraph::new(status_text).style(Style::default().add_modifier(Modifier::REVERSED));
    frame.render_widget(status, right_chunks[1]);
}
