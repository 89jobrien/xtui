use crate::app::{App, Focus};
use ansi_to_tui::IntoText;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, BorderType, List, ListItem, ListState, Paragraph, Tabs, Wrap};

/// Left panel (commands) width as percentage of the terminal.
const CMD_PANEL_PCT: u16 = 25;
/// Right panel (output) width as percentage of the terminal.
const OUTPUT_PANEL_PCT: u16 = 75;
/// Height of the description area in lines.
const DESC_HEIGHT: u16 = 5;

const ACCENT: Color = Color::Cyan;
const HIGHLIGHT_FG: Color = Color::Black;
const HIGHLIGHT_BG: Color = Color::Cyan;
const DIM: Color = Color::DarkGray;
const DESC_COLOR: Color = Color::Yellow;
const STATUS_BG: Color = Color::Blue;
const STATUS_FG: Color = Color::White;

fn border_color(focused: bool, default: Color) -> Color {
    if focused { ACCENT } else { default }
}

pub fn draw(frame: &mut Frame, app: &mut App) {
    // Top-level vertical split: git status bar + main + bottom status
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // git status bar
            Constraint::Min(1),    // main area
            Constraint::Length(1), // bottom status
        ])
        .split(frame.area());

    draw_git_status_bar(frame, app, outer[0]);

    let output_focused = app.focus == Focus::Output;

    // Dep view takes the entire main area — it is not a command runner.
    if app.show_dep_view {
        draw_dep_view(frame, app, outer[1], output_focused);
        draw_bottom_status(frame, app, outer[2]);
        return;
    }

    // Main area: left (tabs + commands) | right (desc + output/status)
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(CMD_PANEL_PCT),
            Constraint::Percentage(OUTPUT_PANEL_PCT),
        ])
        .split(outer[1]);

    // Left side: tab bar + command list
    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(1)])
        .split(main_chunks[0]);

    // Right side: description + output/status
    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(DESC_HEIGHT), Constraint::Min(1)])
        .split(main_chunks[1]);

    app.output_height = right_chunks[1].height.saturating_sub(2);

    let cmd_focused = app.focus == Focus::Commands;

    // Tab bar
    draw_tab_bar(frame, app, left_chunks[0]);

    // Command list
    draw_command_list(frame, app, left_chunks[1], cmd_focused);

    // Description pane
    draw_description(frame, app, right_chunks[0]);

    // Output or Status pane
    if app.show_status_tab {
        draw_status_tab(frame, app, right_chunks[1], output_focused);
    } else {
        draw_output(frame, app, right_chunks[1], output_focused);
    }

    // Bottom status bar
    draw_bottom_status(frame, app, outer[2]);
}

fn draw_git_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    let workspace_name = app
        .workspace
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    let mut spans = vec![Span::styled(
        format!(" {workspace_name}"),
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    )];

    if let Some(ref gs) = app.git_status {
        spans.push(Span::raw(" | "));
        spans.push(Span::styled(&gs.branch, Style::default().fg(ACCENT)));
        spans.push(Span::raw(" | "));
        if gs.dirty {
            spans.push(Span::styled("dirty", Style::default().fg(Color::Red)));
        } else {
            spans.push(Span::styled("clean", Style::default().fg(Color::Green)));
        }
        if gs.ahead > 0 || gs.behind > 0 {
            spans.push(Span::raw(format!(" +{}/-{}", gs.ahead, gs.behind)));
        }
    }

    spans.push(Span::raw(format!(
        " | {} commands",
        app.total_command_count()
    )));

    let bar = Paragraph::new(Line::from(spans))
        .style(Style::default().bg(Color::DarkGray).fg(Color::White));
    frame.render_widget(bar, area);
}

fn draw_tab_bar(frame: &mut Frame, app: &App, area: Rect) {
    if app.tabs.is_empty() {
        return;
    }
    let titles: Vec<Line> = app
        .tabs
        .iter()
        .map(|t| Line::raw(format!(" {} ", t.name)))
        .collect();
    let tabs = Tabs::new(titles)
        .select(app.active_tab)
        .highlight_style(
            Style::default()
                .fg(HIGHLIGHT_FG)
                .bg(HIGHLIGHT_BG)
                .add_modifier(Modifier::BOLD),
        )
        .style(Style::default().fg(Color::White))
        .divider("|");
    frame.render_widget(tabs, area);
}

fn draw_command_list(frame: &mut Frame, app: &App, area: Rect, focused: bool) {
    let cmds = app.current_commands();
    let items: Vec<ListItem> = cmds
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
                .border_style(Style::default().fg(border_color(focused, ACCENT)))
                .title(format!(
                    " {} [{}] ",
                    app.tabs
                        .get(app.active_tab)
                        .map(|t| t.name.as_str())
                        .unwrap_or(""),
                    cmds.len()
                ))
                .title_style(Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
        )
        .highlight_style(Style::default().fg(HIGHLIGHT_FG).bg(HIGHLIGHT_BG));
    let mut list_state = ListState::default().with_selected(Some(app.selected));
    frame.render_stateful_widget(list, area, &mut list_state);
}

fn draw_description(frame: &mut Frame, app: &App, area: Rect) {
    let desc_text = match app.selected_command() {
        Some(cmd) => cmd
            .description
            .as_deref()
            .unwrap_or(&format!("No description for `{}`", cmd.name))
            .to_string(),
        None => "No commands discovered".to_string(),
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
    frame.render_widget(description, area);
}

fn draw_output(frame: &mut Frame, app: &App, area: Rect, focused: bool) {
    let raw_output = app.output.join("\n");
    let output_text = if app.output.is_empty() {
        Text::raw("Press Enter to run selected command")
    } else {
        raw_output
            .as_bytes()
            .into_text()
            .unwrap_or_else(|_| Text::raw(raw_output.clone()))
    };

    let output_border = border_color(focused, DIM);
    let output_title_style = if focused {
        Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD | Modifier::REVERSED)
    } else {
        Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD)
    };

    let mut title = " Output ".to_string();
    if let Some(ref search) = app.search {
        if search.query.is_empty() {
            title = format!(" Search: /{}_ ", search.input_buffer);
        } else {
            title = format!(
                " Output [{}/{}] ",
                if search.matches.is_empty() {
                    0
                } else {
                    search.current + 1
                },
                search.match_count()
            );
        }
    }

    let output = Paragraph::new(output_text)
        .block(
            Block::bordered()
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(output_border))
                .title(title)
                .title_style(output_title_style),
        )
        .wrap(Wrap { trim: false })
        .scroll((app.output_scroll, 0));
    frame.render_widget(output, area);
}

fn draw_status_tab(frame: &mut Frame, app: &App, area: Rect, focused: bool) {
    let mut lines = Vec::new();

    if let Some(ref gs) = app.git_status {
        lines.push(Line::from(vec![
            Span::styled("Branch: ", Style::default().fg(DIM)),
            Span::styled(&gs.branch, Style::default().fg(ACCENT)),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Status: ", Style::default().fg(DIM)),
            if gs.dirty {
                Span::styled("dirty", Style::default().fg(Color::Red))
            } else {
                Span::styled("clean", Style::default().fg(Color::Green))
            },
        ]));
        if gs.ahead > 0 || gs.behind > 0 {
            lines.push(Line::from(format!(
                "Ahead: {} / Behind: {}",
                gs.ahead, gs.behind
            )));
        }
        lines.push(Line::raw(""));
        lines.push(Line::styled(
            "Recent commits:",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ));
        for commit in &gs.recent_commits {
            lines.push(Line::raw(format!("  {commit}")));
        }
        if !gs.diff_stat.is_empty() {
            lines.push(Line::raw(""));
            lines.push(Line::styled(
                "Diff stat:",
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ));
            for line in gs.diff_stat.lines() {
                lines.push(Line::raw(format!("  {line}")));
            }
        }
    } else {
        lines.push(Line::raw("Not a git repository"));
    }

    let status_tab = Paragraph::new(Text::from(lines))
        .block(
            Block::bordered()
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(border_color(focused, DIM)))
                .title(" Status ")
                .title_style(
                    Style::default()
                        .fg(Color::Magenta)
                        .add_modifier(Modifier::BOLD),
                ),
        )
        .wrap(Wrap { trim: false })
        .scroll((app.output_scroll, 0));
    frame.render_widget(status_tab, area);
}

pub(crate) fn format_dep_row(info: &crate::depview::DepInfo) -> String {
    use crate::depview::DepFetchState;
    match &info.state {
        DepFetchState::Loading => format!(
            "{:<30} {:<12} {:<12} {}",
            info.name, info.declared_version, "…", "…"
        ),
        DepFetchState::Local => {
            let repo = info.github_url.as_deref().unwrap_or("—");
            format!(
                "{:<30} {:<12} {:<12} {:<6} {}",
                info.name, info.declared_version, "local", "—", repo
            )
        }
        DepFetchState::Error(e) => format!(
            "{:<30} {:<12} {:<12} err: {}",
            info.name, info.declared_version, "—", e
        ),
        DepFetchState::Ready => {
            let latest = info.crates_io_latest.as_deref().unwrap_or("—");
            let behind = info
                .versions_behind
                .map(|n| {
                    if n == 0 {
                        "✓".to_string()
                    } else {
                        n.to_string()
                    }
                })
                .unwrap_or_else(|| "—".to_string());
            let repo = info.github_url.as_deref().unwrap_or("—");
            format!(
                "{:<30} {:<12} {:<12} {:<6} {}",
                info.name, info.declared_version, latest, behind, repo
            )
        }
    }
}

fn draw_dep_view(frame: &mut Frame, app: &App, area: Rect, focused: bool) {
    use crate::depview::DepFetchState;

    let header = format!(
        "{:<30} {:<12} {:<12} {:<6} {}",
        "Name", "Declared", "Latest", "Behind", "Repo"
    );

    let mut lines: Vec<Line> = vec![
        Line::styled(
            header,
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        ),
        Line::raw(""),
    ];

    for info in &app.dep_infos {
        let row = format_dep_row(info);
        let style = match &info.state {
            DepFetchState::Loading => Style::default().fg(DIM),
            DepFetchState::Local => Style::default().fg(Color::Cyan),
            DepFetchState::Error(_) => Style::default().fg(Color::Red),
            DepFetchState::Ready => {
                if info.versions_behind.unwrap_or(0) > 0 {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default().fg(Color::Green)
                }
            }
        };
        lines.push(Line::styled(row, style));
    }

    let count = app.dep_infos.len();
    let panel = Paragraph::new(Text::from(lines))
        .block(
            Block::bordered()
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(border_color(focused, DIM)))
                .title(format!(" Deps [{}] ", count))
                .title_style(
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
        )
        .wrap(Wrap { trim: false })
        .scroll((app.dep_scroll, 0));
    frame.render_widget(panel, area);
}

fn draw_bottom_status(frame: &mut Frame, app: &App, area: Rect) {
    // Args-input mode overrides the normal status bar.
    if let Some(ref buf) = app.args_input {
        let cmd_name = app
            .selected_command()
            .map(|c| c.name.as_str())
            .unwrap_or("");
        let text = format!(" args> {cmd_name} {buf}_");
        let bar = Paragraph::new(text).style(Style::default().fg(Color::Black).bg(Color::Cyan));
        frame.render_widget(bar, area);
        return;
    }

    let state = if app.task.is_some() {
        "running".to_string()
    } else if let Some(code) = app.exit_code {
        format!("exit: {code}")
    } else {
        "idle".to_string()
    };

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
        Focus::Commands => {
            "Tab:source  1-9:tab  Enter:run  a:args  /:search  s:status  D:deps  P:pipeline"
        }
        Focus::Output => "j/k:scroll  g/G:top/bottom  n/N:search  Esc:back",
    };

    let pipeline_info = if let Some(ref pipe) = app.pipeline {
        match &pipe.status {
            crate::pipeline::PipelineStatus::Running(idx) => {
                format!(" | pipeline [{}/{}]", idx + 1, pipe.step_count())
            }
            crate::pipeline::PipelineStatus::Done(_) => " | pipeline: done".to_string(),
            crate::pipeline::PipelineStatus::Failed(idx, code) => {
                format!(" | pipeline: failed at step {} (exit {code})", idx + 1)
            }
            _ => String::new(),
        }
    } else {
        String::new()
    };

    let status_text = if let Some((msg, _)) = &app.flash_message {
        format!(" {msg}")
    } else {
        format!(" {state}{pipeline_info} | {focus_hint}")
    };
    let status = Paragraph::new(status_text).style(status_style);
    frame.render_widget(status, area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::depview::{DepFetchState, DepInfo};

    #[test]
    fn format_dep_row_ready() {
        let info = DepInfo {
            name: "serde".into(),
            declared_version: "1.0.0".into(),
            crates_io_latest: Some("1.0.200".into()),
            github_url: Some("https://github.com/serde-rs/serde".into()),
            versions_behind: Some(5),
            state: DepFetchState::Ready,
        };
        let row = format_dep_row(&info);
        assert!(row.contains("serde"));
        assert!(row.contains("1.0.0"));
        assert!(row.contains("1.0.200"));
        assert!(row.contains("5"));
    }

    #[test]
    fn format_dep_row_loading() {
        let info = DepInfo {
            name: "tokio".into(),
            declared_version: "1".into(),
            crates_io_latest: None,
            github_url: None,
            versions_behind: None,
            state: DepFetchState::Loading,
        };
        let row = format_dep_row(&info);
        assert!(row.contains("…"));
    }

    #[test]
    fn format_dep_row_zero_behind_shows_checkmark() {
        let info = DepInfo {
            name: "anyhow".into(),
            declared_version: "1.0.95".into(),
            crates_io_latest: Some("1.0.95".into()),
            github_url: None,
            versions_behind: Some(0),
            state: DepFetchState::Ready,
        };
        let row = format_dep_row(&info);
        assert!(row.contains("✓"));
    }

    #[test]
    fn format_dep_row_error() {
        let info = DepInfo {
            name: "bad-crate".into(),
            declared_version: "0.1.0".into(),
            crates_io_latest: None,
            github_url: None,
            versions_behind: None,
            state: DepFetchState::Error("timeout".into()),
        };
        let row = format_dep_row(&info);
        assert!(row.contains("err:"));
        assert!(row.contains("timeout"));
    }

    #[test]
    fn format_dep_row_local() {
        let info = DepInfo {
            name: "mylib".into(),
            declared_version: "0.1.0".into(),
            crates_io_latest: None,
            github_url: Some("https://github.com/me/mylib".into()),
            versions_behind: None,
            state: DepFetchState::Local,
        };
        let row = format_dep_row(&info);
        assert!(row.contains("mylib"));
        assert!(row.contains("local"));
        assert!(row.contains("https://github.com/me/mylib"));
    }
}
