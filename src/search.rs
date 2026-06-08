/// Search state for incremental, case-insensitive substring search over log lines.
pub struct SearchState {
    pub query: String,
    pub matches: Vec<usize>,
    pub current: usize,
    pub active: bool,
    pub input_buffer: String,
}

impl SearchState {
    /// Create a new active search with a fixed query.
    pub fn new(query: &str) -> Self {
        Self {
            query: query.to_string(),
            matches: Vec::new(),
            current: 0,
            active: true,
            input_buffer: String::new(),
        }
    }

    /// Create an empty search state in input-accumulation mode.
    pub fn from_input() -> Self {
        Self {
            query: String::new(),
            matches: Vec::new(),
            current: 0,
            active: true,
            input_buffer: String::new(),
        }
    }

    /// Populate `matches` with indices of lines that contain `query` (case-insensitive).
    pub fn find_matches(&mut self, lines: &[String]) {
        let q = self.query.to_lowercase();
        self.matches = lines
            .iter()
            .enumerate()
            .filter(|(_, line)| line.to_lowercase().contains(&q))
            .map(|(i, _)| i)
            .collect();
        self.current = 0;
    }

    /// Advance to the next match and return its line index. Wraps around. Returns `None` when
    /// there are no matches.
    pub fn next_match(&mut self) -> Option<usize> {
        if self.matches.is_empty() {
            return None;
        }
        self.current = (self.current + 1) % self.matches.len();
        Some(self.matches[self.current])
    }

    /// Go back to the previous match and return its line index. Wraps around. Returns `None`
    /// when there are no matches.
    pub fn prev_match(&mut self) -> Option<usize> {
        if self.matches.is_empty() {
            return None;
        }
        self.current = self
            .current
            .checked_sub(1)
            .unwrap_or(self.matches.len() - 1);
        Some(self.matches[self.current])
    }

    /// Return the current match's line index without advancing.
    pub fn current_line(&self) -> Option<usize> {
        self.matches.get(self.current).copied()
    }

    /// Reset all search state.
    pub fn clear(&mut self) {
        self.query.clear();
        self.matches.clear();
        self.current = 0;
        self.active = false;
        self.input_buffer.clear();
    }

    /// Return the number of matches.
    pub fn match_count(&self) -> usize {
        self.matches.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_finds_matches() {
        let lines = vec![
            "hello world".to_string(),
            "foo bar".to_string(),
            "hello again".to_string(),
        ];
        let mut search = SearchState::new("hello");
        search.find_matches(&lines);
        assert_eq!(search.matches.len(), 2);
        assert_eq!(search.matches[0], 0);
        assert_eq!(search.matches[1], 2);
    }

    #[test]
    fn test_search_case_insensitive() {
        let lines = vec!["Hello World".to_string(), "HELLO".to_string()];
        let mut search = SearchState::new("hello");
        search.find_matches(&lines);
        assert_eq!(search.matches.len(), 2);
    }

    #[test]
    fn test_next_prev_cycling() {
        let lines = vec!["a".into(), "b".into(), "a".into(), "a".into()];
        let mut search = SearchState::new("a");
        search.find_matches(&lines);
        assert_eq!(search.next_match(), Some(2)); // advances from index 0 to 1 in matches vec
        assert_eq!(search.next_match(), Some(3));
        assert_eq!(search.next_match(), Some(0)); // wraps
        assert_eq!(search.prev_match(), Some(3)); // wraps back
    }

    #[test]
    fn test_no_matches() {
        let lines = vec!["foo".to_string()];
        let mut search = SearchState::new("bar");
        search.find_matches(&lines);
        assert_eq!(search.matches.len(), 0);
        assert_eq!(search.next_match(), None);
    }
}
