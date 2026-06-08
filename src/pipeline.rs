/// Pipeline: ordered sequence of named steps executed one at a time.
///
/// Steps are identified by name; the caller resolves names to runnable
/// commands. The pipeline itself only tracks state transitions.

// ── Types ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PipelineStatus {
    Pending,
    /// Index of the currently running step.
    Running(usize),
    /// All steps passed; exit code is always 0.
    Done(i32),
    /// Step at `index` exited with `exit_code`.
    Failed(usize, i32),
}

#[derive(Debug, Clone)]
pub struct PipelineStep {
    pub name: String,
    pub source: String,
}

pub struct Pipeline {
    pub steps: Vec<PipelineStep>,
    pub status: PipelineStatus,
}

// ── Implementation ────────────────────────────────────────────────────────────

impl Pipeline {
    pub fn new(steps: Vec<PipelineStep>) -> Self {
        Self {
            steps,
            status: PipelineStatus::Pending,
        }
    }

    /// Transition to `Running(0)` if there are steps to run.
    pub fn start(&mut self) {
        if !self.steps.is_empty() {
            self.status = PipelineStatus::Running(0);
        }
    }

    /// Return the step currently being run, if any.
    pub fn current_step(&self) -> Option<&PipelineStep> {
        match self.status {
            PipelineStatus::Running(idx) => self.steps.get(idx),
            _ => None,
        }
    }

    /// Record the exit code for the current step and advance.
    ///
    /// - Non-zero exit → `Failed(current, exit_code)`
    /// - Last step passed → `Done(0)`
    /// - Otherwise → `Running(current + 1)`
    ///
    /// Returns `&self.status` for convenience.
    pub fn advance(&mut self, exit_code: i32) -> &PipelineStatus {
        if let PipelineStatus::Running(idx) = self.status {
            if exit_code != 0 {
                self.status = PipelineStatus::Failed(idx, exit_code);
            } else if idx + 1 >= self.steps.len() {
                self.status = PipelineStatus::Done(0);
            } else {
                self.status = PipelineStatus::Running(idx + 1);
            }
        }
        &self.status
    }

    /// True while the pipeline is actively running.
    pub fn is_active(&self) -> bool {
        matches!(self.status, PipelineStatus::Running(_))
    }

    pub fn step_count(&self) -> usize {
        self.steps.len()
    }

    #[cfg(test)]
    /// Parse a `:chain name1 name2 …` command string.
    ///
    /// Returns `None` if the input does not start with `:chain` or contains no
    /// command names after the prefix.
    pub fn parse(input: &str) -> Option<Self> {
        let rest = input.strip_prefix(":chain")?;
        let names: Vec<&str> = rest.split_whitespace().collect();
        if names.is_empty() {
            return None;
        }
        let steps = names
            .into_iter()
            .map(|n| PipelineStep {
                name: n.to_string(),
                source: "unknown".to_string(),
            })
            .collect();
        Some(Self::new(steps))
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_advance_success() {
        let steps = vec![
            PipelineStep {
                name: "check".into(),
                source: "cargo".into(),
            },
            PipelineStep {
                name: "test".into(),
                source: "cargo".into(),
            },
            PipelineStep {
                name: "clippy".into(),
                source: "cargo".into(),
            },
        ];
        let mut pipe = Pipeline::new(steps);
        pipe.start();
        assert_eq!(pipe.status, PipelineStatus::Running(0));
        pipe.advance(0);
        assert_eq!(pipe.status, PipelineStatus::Running(1));
        pipe.advance(0);
        assert_eq!(pipe.status, PipelineStatus::Running(2));
        pipe.advance(0);
        assert_eq!(pipe.status, PipelineStatus::Done(0));
    }

    #[test]
    fn test_pipeline_advance_failure() {
        let steps = vec![
            PipelineStep {
                name: "check".into(),
                source: "cargo".into(),
            },
            PipelineStep {
                name: "test".into(),
                source: "cargo".into(),
            },
        ];
        let mut pipe = Pipeline::new(steps);
        pipe.start();
        pipe.advance(0); // check passes
        pipe.advance(1); // test fails
        assert_eq!(pipe.status, PipelineStatus::Failed(1, 1));
    }

    #[test]
    fn test_pipeline_parse() {
        let pipe = Pipeline::parse(":chain check test clippy");
        assert!(pipe.is_some());
        let pipe = pipe.unwrap();
        assert_eq!(pipe.steps.len(), 3);
        assert_eq!(pipe.steps[0].name, "check");
        assert_eq!(pipe.steps[2].name, "clippy");
    }

    #[test]
    fn test_pipeline_parse_invalid() {
        assert!(Pipeline::parse("not a chain").is_none());
        assert!(Pipeline::parse(":chain").is_none());
        assert!(Pipeline::parse(":chain ").is_none());
    }

    #[test]
    fn test_pipeline_empty() {
        let pipe = Pipeline::new(vec![]);
        assert!(!pipe.is_active());
        assert!(pipe.current_step().is_none());
    }

    #[test]
    fn test_advance_on_pending_is_noop() {
        let steps = vec![PipelineStep {
            name: "check".into(),
            source: "cargo".into(),
        }];
        let mut pipe = Pipeline::new(steps);
        // Don't call start — status is Pending
        pipe.advance(0);
        assert_eq!(pipe.status, PipelineStatus::Pending);
    }

    #[test]
    fn test_advance_on_done_is_noop() {
        let steps = vec![PipelineStep {
            name: "check".into(),
            source: "cargo".into(),
        }];
        let mut pipe = Pipeline::new(steps);
        pipe.start();
        pipe.advance(0); // -> Done
        assert_eq!(pipe.status, PipelineStatus::Done(0));
        pipe.advance(0); // noop
        assert_eq!(pipe.status, PipelineStatus::Done(0));
    }

    #[test]
    fn test_advance_on_failed_is_noop() {
        let steps = vec![PipelineStep {
            name: "check".into(),
            source: "cargo".into(),
        }];
        let mut pipe = Pipeline::new(steps);
        pipe.start();
        pipe.advance(1); // -> Failed
        assert_eq!(pipe.status, PipelineStatus::Failed(0, 1));
        pipe.advance(0); // noop
        assert_eq!(pipe.status, PipelineStatus::Failed(0, 1));
    }

    #[test]
    fn test_single_step_success() {
        let steps = vec![PipelineStep {
            name: "check".into(),
            source: "cargo".into(),
        }];
        let mut pipe = Pipeline::new(steps);
        pipe.start();
        pipe.advance(0);
        assert_eq!(pipe.status, PipelineStatus::Done(0));
    }

    #[test]
    fn test_single_step_failure() {
        let steps = vec![PipelineStep {
            name: "check".into(),
            source: "cargo".into(),
        }];
        let mut pipe = Pipeline::new(steps);
        pipe.start();
        pipe.advance(42);
        assert_eq!(pipe.status, PipelineStatus::Failed(0, 42));
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn advance_never_panics(exit_codes in proptest::collection::vec(-128i32..128, 0..20)) {
            let steps: Vec<PipelineStep> = (0..exit_codes.len())
                .map(|i| PipelineStep {
                    name: format!("step{i}"),
                    source: "test".into(),
                })
                .collect();
            let mut pipe = Pipeline::new(steps);
            pipe.start();
            for code in &exit_codes {
                pipe.advance(*code);
            }
        }
    }
}
