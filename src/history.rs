use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Maximum number of history entries retained per project.
pub const HISTORY_ENTRY_CAP: usize = 50;
/// Maximum number of log files retained per project.
pub const LOG_FILE_CAP: usize = 100;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub command: String,
    pub source: String,
    pub exit_code: i32,
    pub timestamp: String,
    pub duration_secs: u64,
}

/// Returns the default history directory: `~/.config/xtui/history/`.
pub fn history_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from(".config"))
        .join("xtui")
        .join("history")
}

/// Appends `entry` to `<base>/<project>.json`, capping the list at 50 entries.
///
/// The file stores a JSON array. If the file does not exist it is created.
pub fn save_entry(base: &Path, project: &str, entry: &HistoryEntry) -> Result<()> {
    fs::create_dir_all(base)?;
    let file = base.join(format!("{project}.json"));
    let mut entries = load_history(base, project)?;
    entries.push(entry.clone());
    if entries.len() > HISTORY_ENTRY_CAP {
        let drop = entries.len() - HISTORY_ENTRY_CAP;
        entries.drain(..drop);
    }
    let json = serde_json::to_string_pretty(&entries)?;
    fs::write(&file, json)?;
    Ok(())
}

/// Loads the history for `project` from `<base>/<project>.json`.
///
/// Returns an empty `Vec` when the file does not exist.
pub fn load_history(base: &Path, project: &str) -> Result<Vec<HistoryEntry>> {
    let file = base.join(format!("{project}.json"));
    if !file.exists() {
        return Ok(Vec::new());
    }
    let contents = fs::read_to_string(&file)?;
    let entries: Vec<HistoryEntry> = serde_json::from_str(&contents)?;
    Ok(entries)
}

/// Writes `output` lines to `<base>/<project>/<timestamp>-<command>.log`.
///
/// The directory is created if it does not exist. The timestamp is the current
/// UTC time formatted as `YYYYMMDDTHHMMSSZ`.
pub fn save_output(base: &Path, project: &str, command: &str, output: &[String]) -> Result<()> {
    let log_dir = base.join(project);
    fs::create_dir_all(&log_dir)?;

    let timestamp = utc_timestamp();
    let safe_cmd: String = command
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let filename = format!("{timestamp}-{safe_cmd}.log");

    fs::write(log_dir.join(filename), output.join("\n"))?;
    Ok(())
}

/// Deletes the oldest log files in `<base>/<project>/` when there are more than 100.
///
/// Log files are sorted by filename so the lexicographic order matches chronological
/// order (the filename prefix is a timestamp).
pub fn prune_logs(base: &Path, project: &str) -> Result<()> {
    let log_dir = base.join(project);
    if !log_dir.exists() {
        return Ok(());
    }

    let mut files: Vec<PathBuf> = fs::read_dir(&log_dir)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_file())
        .collect();

    if files.len() <= LOG_FILE_CAP {
        return Ok(());
    }

    // Sort ascending — oldest (smallest name) first.
    files.sort();
    let to_delete = files.len() - LOG_FILE_CAP;
    for path in files.iter().take(to_delete) {
        fs::remove_file(path)?;
    }
    Ok(())
}

/// Returns the current UTC time as a filesystem-safe string: `YYYYMMDDTHHMMSSZ`.
fn utc_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let sec = secs % 60;
    let min = (secs / 60) % 60;
    let hour = (secs / 3600) % 24;
    let days = secs / 86400;

    let (year, month, day) = days_to_ymd(days);
    format!("{year:04}{month:02}{day:02}T{hour:02}{min:02}{sec:02}Z")
}

/// Converts days since the Unix epoch to `(year, month, day)`.
///
/// Uses the algorithm from <https://howardhinnant.github.io/date_algorithms.html>.
// qual:allow(complexity) reason: "Gregorian calendar algorithm — magic numbers are well-defined algorithm constants"
fn days_to_ymd(days: u64) -> (u64, u64, u64) {
    let z = days as i64 + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y } as u64;
    (y, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_history_round_trip() {
        let tmp = std::env::temp_dir().join("xtui-test-history");
        let _ = std::fs::remove_dir_all(&tmp);
        let entry = HistoryEntry {
            command: "check".into(),
            source: "cargo".into(),
            exit_code: 0,
            timestamp: "2026-06-08T12:00:00Z".into(),
            duration_secs: 5,
        };
        save_entry(&tmp, "myproj", &entry).unwrap();
        let entries = load_history(&tmp, "myproj").unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].command, "check");
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_load_nonexistent_returns_empty() {
        let tmp = std::env::temp_dir().join("xtui-test-history-none");
        let entries = load_history(&tmp, "nope").unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn test_save_output_creates_log() {
        let tmp = std::env::temp_dir().join("xtui-test-output");
        let _ = std::fs::remove_dir_all(&tmp);
        let lines = vec!["line1".into(), "line2".into()];
        save_output(&tmp, "myproj", "check", &lines).unwrap();
        let log_dir = tmp.join("myproj");
        let entries: Vec<_> = std::fs::read_dir(&log_dir).unwrap().collect();
        assert_eq!(entries.len(), 1);
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_history_caps_at_50() {
        let tmp = std::env::temp_dir().join("xtui-test-history-cap");
        let _ = std::fs::remove_dir_all(&tmp);
        for i in 0..60 {
            let entry = HistoryEntry {
                command: format!("cmd{i}"),
                source: "test".into(),
                exit_code: 0,
                timestamp: format!("2026-06-08T12:{i:02}:00Z"),
                duration_secs: 1,
            };
            save_entry(&tmp, "myproj", &entry).unwrap();
        }
        let entries = load_history(&tmp, "myproj").unwrap();
        assert_eq!(entries.len(), 50);
        assert_eq!(entries.last().unwrap().command, "cmd59");
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_prune_at_exactly_100() {
        let tmp = std::env::temp_dir().join("xtui-test-prune-100");
        let _ = std::fs::remove_dir_all(&tmp);
        let log_dir = tmp.join("myproj");
        std::fs::create_dir_all(&log_dir).unwrap();
        for i in 0..100 {
            std::fs::write(log_dir.join(format!("{i:04}.log")), "x").unwrap();
        }
        prune_logs(&tmp, "myproj").unwrap();
        let count = std::fs::read_dir(&log_dir).unwrap().count();
        assert_eq!(count, 100); // no pruning
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_prune_at_101() {
        let tmp = std::env::temp_dir().join("xtui-test-prune-101");
        let _ = std::fs::remove_dir_all(&tmp);
        let log_dir = tmp.join("myproj");
        std::fs::create_dir_all(&log_dir).unwrap();
        for i in 0..101 {
            std::fs::write(log_dir.join(format!("{i:04}.log")), "x").unwrap();
        }
        prune_logs(&tmp, "myproj").unwrap();
        let count = std::fs::read_dir(&log_dir).unwrap().count();
        assert_eq!(count, 100);
        // Oldest file (0000.log) should be gone
        assert!(!log_dir.join("0000.log").exists());
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_save_entry_special_chars() {
        let tmp = std::env::temp_dir().join("xtui-test-special");
        let _ = std::fs::remove_dir_all(&tmp);
        let entry = HistoryEntry {
            command: "check".into(),
            source: "cargo".into(),
            exit_code: 0,
            timestamp: String::new(),
            duration_secs: 1,
        };
        // Project name with spaces and unicode
        save_entry(&tmp, "my project", &entry).unwrap();
        let entries = load_history(&tmp, "my project").unwrap();
        assert_eq!(entries.len(), 1);
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_days_to_ymd_epoch() {
        let (y, m, d) = days_to_ymd(0);
        assert_eq!((y, m, d), (1970, 1, 1));
    }

    #[test]
    fn test_days_to_ymd_leap_day() {
        // 2024-02-29: days since epoch
        // 2024-01-01 = 19723 days since epoch
        // Jan: 31, Feb 1-29: 29 -> 31 + 28 = 59 days into year
        // 19723 + 59 = 19782
        let (y, m, d) = days_to_ymd(19782);
        assert_eq!((y, m, d), (2024, 2, 29));
    }

    #[test]
    fn test_days_to_ymd_today() {
        // 2026-06-08
        // 2026-01-01 = 20454 days since epoch
        // Jan:31 Feb:28 Mar:31 Apr:30 May:31 Jun:1-8 = 31+28+31+30+31+8 = 159
        // But day 0 of year is Jan 1, so offset is 158
        // 20454 + 158 = 20612
        let (y, m, d) = days_to_ymd(20612);
        assert_eq!((y, m, d), (2026, 6, 8));
    }
}
