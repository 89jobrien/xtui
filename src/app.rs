use crate::history::{self, HistoryEntry};
use crate::pipeline::{Pipeline, PipelineStep};
use crate::runner::{self, RunningTask};
use crate::search::SearchState;
use crate::source::{CommandSource, SourceCommand, all_sources};
use crate::status::{self, GitStatus};
use crate::ui;
use anyhow::Result;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::crossterm::execute;
use ratatui::crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use std::io;
use std::path::PathBuf;

fn base64_encode(input: &[u8]) -> String {
    use std::fmt::Write;
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let n = (b0 << 16) | (b1 << 8) | b2;
        let _ = write!(out, "{}", CHARS[(n >> 18 & 63) as usize] as char);
        let _ = write!(out, "{}", CHARS[(n >> 12 & 63) as usize] as char);
        if chunk.len() > 1 {
            let _ = write!(out, "{}", CHARS[(n >> 6 & 63) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            let _ = write!(out, "{}", CHARS[(n & 63) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Commands,
    Output,
}

/// A tab grouping commands from a single source.
pub struct SourceTab {
    pub name: String,
    pub commands: Vec<SourceCommand>,
}

pub struct App {
    pub workspace: PathBuf,
    pub sources: Vec<Box<dyn CommandSource>>,
    pub tabs: Vec<SourceTab>,
    pub active_tab: usize,
    pub selected: usize,
    pub output: Vec<String>,
    pub task: Option<RunningTask>,
    pub exit_code: Option<i32>,
    pub should_quit: bool,
    pub flash_message: Option<(String, std::time::Instant)>,
    pub focus: Focus,
    pub output_scroll: u16,
    pub output_height: u16,
    pub git_status: Option<GitStatus>,
    pub search: Option<SearchState>,
    pub show_status_tab: bool,
    pub pipeline: Option<Pipeline>,
    pub run_start: Option<std::time::Instant>,
}

impl App {
    pub fn new(workspace: PathBuf) -> Self {
        let sources = all_sources();
        let git_status = status::collect_git_status(&workspace);

        let mut app = Self {
            workspace,
            sources,
            tabs: Vec::new(),
            active_tab: 0,
            selected: 0,
            output: Vec::new(),
            task: None,
            exit_code: None,
            should_quit: false,
            flash_message: None,
            focus: Focus::Commands,
            output_scroll: 0,
            output_height: 0,
            git_status,
            search: None,
            show_status_tab: false,
            pipeline: None,
            run_start: None,
        };
        app.discover_all();
        app
    }

    fn discover_all(&mut self) {
        self.tabs.clear();
        for source in &self.sources {
            match source.discover(&self.workspace) {
                Ok(cmds) if !cmds.is_empty() => {
                    self.tabs.push(SourceTab {
                        name: source.name().to_string(),
                        commands: cmds,
                    });
                }
                _ => {}
            }
        }
        self.active_tab = 0;
        self.selected = 0;
    }

    /// Returns the commands in the currently active tab.
    pub fn current_commands(&self) -> &[SourceCommand] {
        self.tabs
            .get(self.active_tab)
            .map(|t| t.commands.as_slice())
            .unwrap_or(&[])
    }

    /// Returns the currently selected command, if any.
    pub fn selected_command(&self) -> Option<&SourceCommand> {
        self.current_commands().get(self.selected)
    }

    pub fn total_command_count(&self) -> usize {
        self.tabs.iter().map(|t| t.commands.len()).sum()
    }

    pub fn next(&mut self) {
        let len = self.current_commands().len();
        if len > 0 && self.selected + 1 < len {
            self.selected += 1;
        }
    }

    pub fn previous(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    fn next_tab(&mut self) {
        if !self.tabs.is_empty() {
            self.active_tab = (self.active_tab + 1) % self.tabs.len();
            self.selected = 0;
        }
    }

    fn prev_tab(&mut self) {
        if !self.tabs.is_empty() {
            self.active_tab = self
                .active_tab
                .checked_sub(1)
                .unwrap_or(self.tabs.len() - 1);
            self.selected = 0;
        }
    }

    fn switch_tab(&mut self, idx: usize) {
        if idx < self.tabs.len() {
            self.active_tab = idx;
            self.selected = 0;
        }
    }

    fn scroll_output_down(&mut self) {
        let max = (self.output.len() as u16).saturating_sub(self.output_height);
        if self.output_scroll < max {
            self.output_scroll += 1;
        }
    }

    fn scroll_output_up(&mut self) {
        self.output_scroll = self.output_scroll.saturating_sub(1);
    }

    fn scroll_output_to_bottom(&mut self) {
        let max = (self.output.len() as u16).saturating_sub(self.output_height);
        self.output_scroll = max;
    }

    async fn run_selected(&mut self) -> Result<()> {
        if self.task.is_some() {
            return Ok(());
        }
        let Some(cmd) = self.selected_command().cloned() else {
            return Ok(());
        };
        self.output.clear();
        self.output_scroll = 0;
        self.exit_code = None;
        self.run_start = Some(std::time::Instant::now());
        let task = runner::run_source_command(&self.workspace, &cmd).await?;
        self.task = Some(task);
        Ok(())
    }

    async fn cancel(&mut self) {
        if let Some(ref mut task) = self.task {
            task.cancel().await;
        }
        self.task = None;
        self.pipeline = None;
    }

    fn poll_output(&mut self) {
        let mut finished = None;
        if let Some(ref mut task) = self.task {
            let prev_len = self.output.len();
            task.poll_lines(&mut self.output);
            if self.output.len() > 10_000 {
                self.output.drain(..1000);
            }
            if self.output.len() != prev_len && self.focus == Focus::Commands {
                let max = (self.output.len() as u16).saturating_sub(self.output_height);
                self.output_scroll = max;
            }
            if let Some(code) = task.try_exit_code() {
                finished = Some(code);
            }
        }
        if let Some(code) = finished {
            self.exit_code = Some(code);
            self.task = None;
            self.on_command_finished(code);
        }
    }

    fn on_command_finished(&mut self, exit_code: i32) {
        // Save history
        let cmd = self.selected_command().cloned();
        if let Some(cmd) = cmd {
            let duration = self.run_start.map(|s| s.elapsed().as_secs()).unwrap_or(0);
            let project_name = self
                .workspace
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            let entry = HistoryEntry {
                command: cmd.name.clone(),
                source: cmd.source.clone(),
                exit_code,
                timestamp: String::new(),
                duration_secs: duration,
            };
            let base = history::history_dir();
            let _ = history::save_entry(&base, &project_name, &entry);
            let _ = history::save_output(&base, &project_name, &cmd.name, &self.output);
            let _ = history::prune_logs(&base, &project_name);
        }
        self.run_start = None;

        // Advance pipeline if active
        if let Some(ref mut pipe) = self.pipeline {
            pipe.advance(exit_code);
            // If pipeline is still running, we need to start the next step
            // This will be handled in the main loop
        }
    }

    /// If a pipeline is active and the current step finished, start the next one.
    async fn advance_pipeline(&mut self) -> Result<()> {
        let should_start_next = if let Some(ref pipe) = self.pipeline {
            self.task.is_none() && pipe.is_active()
        } else {
            false
        };

        if should_start_next {
            if let Some(ref pipe) = self.pipeline {
                if let Some(step) = pipe.current_step() {
                    let cmd = SourceCommand {
                        name: step.name.clone(),
                        source: step.source.clone(),
                        description: None,
                    };
                    self.output
                        .push(format!("--- [pipeline] running: {} ---", cmd.name));
                    self.run_start = Some(std::time::Instant::now());
                    let task = runner::run_source_command(&self.workspace, &cmd).await?;
                    self.task = Some(task);
                }
            }
        }
        Ok(())
    }

    fn copy_output(&mut self) {
        use std::io::Write;
        if self.output.is_empty() {
            return;
        }
        let text = self.output.join("\n");
        let encoded = base64_encode(text.as_bytes());
        let _ = write!(io::stdout(), "\x1b]52;c;{encoded}\x07");
        let _ = io::stdout().flush();
        let lines = self.output.len();
        self.flash_message = Some((
            format!("Copied {lines} lines to clipboard"),
            std::time::Instant::now(),
        ));
    }

    fn refresh_commands(&mut self) {
        self.discover_all();
        self.git_status = status::collect_git_status(&self.workspace);
    }

    fn start_search(&mut self) {
        self.search = Some(SearchState::from_input());
        self.focus = Focus::Output;
    }

    fn handle_search_input(&mut self, code: KeyCode) {
        let Some(ref mut search) = self.search else {
            return;
        };
        match code {
            KeyCode::Enter => {
                search.query = search.input_buffer.clone();
                search.find_matches(&self.output);
                if let Some(line) = search.current_line() {
                    self.output_scroll = line as u16;
                }
            }
            KeyCode::Char(ch) => {
                search.input_buffer.push(ch);
            }
            KeyCode::Backspace => {
                search.input_buffer.pop();
            }
            KeyCode::Esc => {
                self.search = None;
            }
            _ => {}
        }
    }

    fn search_next(&mut self) {
        if let Some(ref mut search) = self.search {
            if let Some(line) = search.next_match() {
                self.output_scroll = line as u16;
            }
        }
    }

    fn search_prev(&mut self) {
        if let Some(ref mut search) = self.search {
            if let Some(line) = search.prev_match() {
                self.output_scroll = line as u16;
            }
        }
    }

    fn start_pipeline_from_selected(&mut self) {
        // Build a pipeline from all commands in the current tab
        let steps: Vec<PipelineStep> = self
            .current_commands()
            .iter()
            .map(|c| PipelineStep {
                name: c.name.clone(),
                source: c.source.clone(),
            })
            .collect();
        if steps.is_empty() {
            return;
        }
        let mut pipe = Pipeline::new(steps);
        pipe.start();
        self.pipeline = Some(pipe);
        self.output.clear();
        self.output_scroll = 0;
        self.exit_code = None;
    }

    pub async fn run(&mut self) -> Result<()> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        while !self.should_quit {
            if let Some((_, at)) = &self.flash_message
                && at.elapsed() > std::time::Duration::from_secs(2)
            {
                self.flash_message = None;
            }

            terminal.draw(|frame| ui::draw(frame, &mut *self))?;
            self.poll_output();
            self.advance_pipeline().await?;

            if event::poll(std::time::Duration::from_millis(50))?
                && let Event::Key(key) = event::read()?
            {
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                // If in search input mode, route all keys to search handler
                if self
                    .search
                    .as_ref()
                    .is_some_and(|s| s.query.is_empty() && s.active)
                {
                    self.handle_search_input(key.code);
                    continue;
                }

                match key.code {
                    KeyCode::Char('q') => {
                        self.cancel().await;
                        self.should_quit = true;
                    }
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        if self.task.is_some() {
                            self.cancel().await;
                        } else {
                            self.cancel().await;
                            self.should_quit = true;
                        }
                    }
                    KeyCode::Esc => {
                        if self.search.is_some() {
                            self.search = None;
                        } else if self.task.is_some() {
                            self.cancel().await;
                        } else if self.focus == Focus::Output {
                            self.focus = Focus::Commands;
                        }
                    }
                    KeyCode::Tab | KeyCode::BackTab => {
                        if self.focus == Focus::Commands {
                            // Tab cycles source tabs when in Commands focus
                            if key.code == KeyCode::Tab {
                                self.next_tab();
                            } else {
                                self.prev_tab();
                            }
                        } else {
                            self.focus = Focus::Commands;
                        }
                    }
                    KeyCode::Char(ch @ '1'..='9') if self.focus == Focus::Commands => {
                        let idx = (ch as usize) - ('1' as usize);
                        self.switch_tab(idx);
                    }
                    KeyCode::Char('j') | KeyCode::Down => match self.focus {
                        Focus::Commands => self.next(),
                        Focus::Output => self.scroll_output_down(),
                    },
                    KeyCode::Char('k') | KeyCode::Up => match self.focus {
                        Focus::Commands => self.previous(),
                        Focus::Output => self.scroll_output_up(),
                    },
                    KeyCode::Char('g') if self.focus == Focus::Output => {
                        self.output_scroll = 0;
                    }
                    KeyCode::Char('G') if self.focus == Focus::Output => {
                        self.scroll_output_to_bottom();
                    }
                    KeyCode::Enter => {
                        if self.focus == Focus::Output {
                            self.focus = Focus::Commands;
                        } else {
                            self.run_selected().await?;
                        }
                    }
                    KeyCode::Char('r') => self.refresh_commands(),
                    KeyCode::Char('c') => self.copy_output(),
                    KeyCode::Char('s') => {
                        self.show_status_tab = !self.show_status_tab;
                        if self.show_status_tab {
                            self.focus = Focus::Output;
                        }
                    }
                    KeyCode::Char('/') => self.start_search(),
                    KeyCode::Char('n') if self.focus == Focus::Output => self.search_next(),
                    KeyCode::Char('N') if self.focus == Focus::Output => self.search_prev(),
                    KeyCode::Char('o') => {
                        self.focus = Focus::Output;
                    }
                    KeyCode::Char('P') => self.start_pipeline_from_selected(),
                    _ => {}
                }
            }
        }

        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_navigate() {
        let mut app = App::new(PathBuf::from(env!("CARGO_MANIFEST_DIR")));
        // Should have at least xtask and cargo tabs
        assert!(!app.tabs.is_empty());
        let len = app.current_commands().len();
        assert!(len > 0);
        app.next();
        if len > 1 {
            assert_eq!(app.selected, 1);
        }
        app.previous();
        assert_eq!(app.selected, 0);
    }

    #[test]
    fn test_tab_switching() {
        let mut app = App::new(PathBuf::from(env!("CARGO_MANIFEST_DIR")));
        if app.tabs.len() >= 2 {
            assert_eq!(app.active_tab, 0);
            app.next_tab();
            assert_eq!(app.active_tab, 1);
            assert_eq!(app.selected, 0);
            app.prev_tab();
            assert_eq!(app.active_tab, 0);
        }
    }

    #[test]
    fn test_total_command_count() {
        let app = App::new(PathBuf::from(env!("CARGO_MANIFEST_DIR")));
        let total = app.total_command_count();
        // At minimum: 4 cargo commands + some xtask commands
        assert!(total >= 4);
    }

    #[test]
    fn test_switch_tab_by_index() {
        let mut app = App::new(PathBuf::from(env!("CARGO_MANIFEST_DIR")));
        if app.tabs.len() >= 2 {
            app.switch_tab(1);
            assert_eq!(app.active_tab, 1);
            assert_eq!(app.selected, 0);
            // Out-of-bounds index is a no-op
            app.switch_tab(999);
            assert_eq!(app.active_tab, 1);
        }
    }

    #[test]
    fn test_sources_grouped_by_tab() {
        let app = App::new(PathBuf::from(env!("CARGO_MANIFEST_DIR")));
        // Each tab should have a unique source name
        let names: Vec<&str> = app.tabs.iter().map(|t| t.name.as_str()).collect();
        let mut deduped = names.clone();
        deduped.sort();
        deduped.dedup();
        assert_eq!(names.len(), deduped.len(), "duplicate tab names: {names:?}");
        // xtui has Cargo.toml so cargo tab must exist
        assert!(names.contains(&"cargo"), "missing cargo tab in {names:?}");
    }

    #[test]
    fn test_selected_command_returns_correct_source() {
        let app = App::new(PathBuf::from(env!("CARGO_MANIFEST_DIR")));
        if let Some(cmd) = app.selected_command() {
            let tab_name = &app.tabs[app.active_tab].name;
            assert_eq!(&cmd.source, tab_name);
        }
    }

    #[test]
    fn test_scroll_bounds() {
        let mut app = App::new(PathBuf::from(env!("CARGO_MANIFEST_DIR")));
        app.output_height = 10;
        // No output — scroll should stay at 0
        app.scroll_output_down();
        assert_eq!(app.output_scroll, 0);
        // Add output exceeding height
        app.output = (0..25).map(|i| format!("line {i}")).collect();
        app.scroll_output_to_bottom();
        assert_eq!(app.output_scroll, 15); // 25 - 10
        app.scroll_output_up();
        assert_eq!(app.output_scroll, 14);
        // Scroll up past 0 saturates
        app.output_scroll = 0;
        app.scroll_output_up();
        assert_eq!(app.output_scroll, 0);
    }

    #[test]
    fn test_search_integration() {
        let mut app = App::new(PathBuf::from(env!("CARGO_MANIFEST_DIR")));
        app.output = vec!["hello world".into(), "foo bar".into(), "hello again".into()];
        app.start_search();
        assert!(app.search.is_some());
        assert_eq!(app.focus, Focus::Output);
        // Type query
        app.handle_search_input(KeyCode::Char('h'));
        app.handle_search_input(KeyCode::Char('e'));
        app.handle_search_input(KeyCode::Char('l'));
        app.handle_search_input(KeyCode::Enter);
        let search = app.search.as_ref().unwrap();
        assert_eq!(search.match_count(), 2);
        // Cycling
        app.search_next();
        assert_eq!(app.output_scroll, 2);
        app.search_prev();
        assert_eq!(app.output_scroll, 0);
    }

    #[test]
    fn test_pipeline_construction() {
        let mut app = App::new(PathBuf::from(env!("CARGO_MANIFEST_DIR")));
        assert!(!app.current_commands().is_empty());
        app.start_pipeline_from_selected();
        assert!(app.pipeline.is_some());
        let pipe = app.pipeline.as_ref().unwrap();
        assert_eq!(pipe.step_count(), app.current_commands().len());
    }

    #[test]
    fn test_focus_toggle() {
        let mut app = App::new(PathBuf::from(env!("CARGO_MANIFEST_DIR")));
        assert_eq!(app.focus, Focus::Commands);
        app.focus = Focus::Output;
        assert_eq!(app.focus, Focus::Output);
    }

    #[test]
    fn test_base64_encode_rfc4648_vectors() {
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"f"), "Zg==");
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        assert_eq!(base64_encode(b"foo"), "Zm9v");
        assert_eq!(base64_encode(b"foob"), "Zm9vYg==");
        assert_eq!(base64_encode(b"fooba"), "Zm9vYmE=");
        assert_eq!(base64_encode(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn test_flash_message() {
        let mut app = App::new(PathBuf::from(env!("CARGO_MANIFEST_DIR")));
        assert!(app.flash_message.is_none());
        app.flash_message = Some(("test".into(), std::time::Instant::now()));
        assert!(app.flash_message.is_some());
    }

    #[test]
    fn test_refresh_commands_preserves_structure() {
        let mut app = App::new(PathBuf::from(env!("CARGO_MANIFEST_DIR")));
        let tab_count = app.tabs.len();
        let total = app.total_command_count();
        app.refresh_commands();
        assert_eq!(app.tabs.len(), tab_count);
        assert_eq!(app.total_command_count(), total);
    }
}
