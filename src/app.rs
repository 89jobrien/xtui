use crate::discover::XtaskCommand;
use crate::runner::{self, RunningTask};
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

pub struct App {
    pub workspace: PathBuf,
    pub commands: Vec<XtaskCommand>,
    pub selected: usize,
    pub output: Vec<String>,
    pub task: Option<RunningTask>,
    pub exit_code: Option<i32>,
    pub should_quit: bool,
    pub flash_message: Option<(String, std::time::Instant)>,
    pub focus: Focus,
    pub output_scroll: u16,
    pub output_height: u16,
}

impl App {
    pub fn new(workspace: PathBuf, commands: Vec<XtaskCommand>) -> Self {
        Self {
            workspace,
            commands,
            selected: 0,
            output: Vec::new(),
            task: None,
            exit_code: None,
            should_quit: false,
            flash_message: None,
            focus: Focus::Commands,
            output_scroll: 0,
            output_height: 0,
        }
    }

    pub fn next(&mut self) {
        if self.selected + 1 < self.commands.len() {
            self.selected += 1;
        }
    }

    pub fn previous(&mut self) {
        self.selected = self.selected.saturating_sub(1);
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
        if self.task.is_some() || self.commands.is_empty() {
            return Ok(());
        }
        let cmd = &self.commands[self.selected];
        self.output.clear();
        self.exit_code = None;
        let task = runner::run_xtask(&self.workspace, &cmd.name).await?;
        self.task = Some(task);
        Ok(())
    }

    async fn cancel(&mut self) {
        if let Some(ref mut task) = self.task {
            task.cancel().await;
        }
        self.task = None;
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
        }
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

    async fn refresh_commands(&mut self) -> Result<()> {
        let cmds = crate::discover::discover_commands(&self.workspace).await?;
        self.commands = cmds;
        self.selected = 0;
        Ok(())
    }

    pub async fn run(&mut self) -> Result<()> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        while !self.should_quit {
            // Expire flash message after 2 seconds
            if let Some((_, at)) = &self.flash_message
                && at.elapsed() > std::time::Duration::from_secs(2)
            {
                self.flash_message = None;
            }

            terminal.draw(|frame| ui::draw(frame, &mut *self))?;
            self.poll_output();

            if event::poll(std::time::Duration::from_millis(50))?
                && let Event::Key(key) = event::read()?
            {
                if key.kind != KeyEventKind::Press {
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
                        if self.task.is_some() {
                            self.cancel().await;
                        } else if self.focus == Focus::Output {
                            self.focus = Focus::Commands;
                        }
                    }
                    KeyCode::Tab | KeyCode::BackTab => {
                        self.focus = match self.focus {
                            Focus::Commands => Focus::Output,
                            Focus::Output => Focus::Commands,
                        };
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
                    KeyCode::Enter => self.run_selected().await?,
                    KeyCode::Char('r') => self.refresh_commands().await?,
                    KeyCode::Char('c') => self.copy_output(),
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
        let cmds = vec![
            XtaskCommand {
                name: "verify".into(),
                description: None,
            },
            XtaskCommand {
                name: "lint".into(),
                description: None,
            },
            XtaskCommand {
                name: "fix".into(),
                description: None,
            },
        ];
        let mut app = App::new("/tmp".into(), cmds);
        assert_eq!(app.selected, 0);
        app.next();
        assert_eq!(app.selected, 1);
        app.next();
        assert_eq!(app.selected, 2);
        app.next();
        assert_eq!(app.selected, 2); // clamp at end
        app.previous();
        assert_eq!(app.selected, 1);
    }
}
