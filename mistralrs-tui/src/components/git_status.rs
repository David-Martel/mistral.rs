//! Git status integration for the TUI status bar
//!
//! Provides real-time git repository status information including branch name,
//! ahead/behind counts, and file change statistics.

use ratatui::{
    style::{Color, Style},
    text::Span,
};
use std::path::Path;
use std::process::Command;

/// Git repository status information
#[derive(Debug, Clone, Default)]
pub struct GitStatus {
    /// Current branch name
    pub branch: Option<String>,
    /// Number of commits ahead of remote
    pub ahead: usize,
    /// Number of commits behind remote
    pub behind: usize,
    /// Number of modified files
    pub modified: usize,
    /// Number of staged files
    pub staged: usize,
    /// Number of untracked files
    pub untracked: usize,
    /// Whether the current path is in a git repository
    pub is_repo: bool,
}

impl GitStatus {
    /// Check if the repository is clean (no changes)
    pub fn is_clean(&self) -> bool {
        self.modified == 0 && self.staged == 0 && self.untracked == 0
    }

    /// Check if the branch is synchronized with remote
    pub fn is_synced(&self) -> bool {
        self.ahead == 0 && self.behind == 0
    }
}

/// Git status provider that queries git via shell commands
pub struct GitStatusProvider {
    repo_path: std::path::PathBuf,
    cached_status: GitStatus,
}

impl GitStatusProvider {
    /// Create a new git status provider for the given repository path
    pub fn new(repo_path: &Path) -> Self {
        let mut provider = Self {
            repo_path: repo_path.to_path_buf(),
            cached_status: GitStatus::default(),
        };
        provider.refresh();
        provider
    }

    /// Check if the given path is in a git repository
    pub fn is_in_repo(&self) -> bool {
        self.cached_status.is_repo
    }

    /// Get the cached git status
    pub fn status(&self) -> &GitStatus {
        &self.cached_status
    }

    /// Refresh git status by querying git commands
    pub fn refresh(&mut self) {
        // Check if we're in a git repo
        if !self.check_is_repo() {
            self.cached_status = GitStatus::default();
            return;
        }

        let mut status = GitStatus {
            is_repo: true,
            ..Default::default()
        };

        // Get branch name
        status.branch = self.get_branch_name();

        // Get ahead/behind counts
        let (ahead, behind) = self.get_ahead_behind();
        status.ahead = ahead;
        status.behind = behind;

        // Get file status counts
        let (modified, staged, untracked) = self.get_file_status();
        status.modified = modified;
        status.staged = staged;
        status.untracked = untracked;

        self.cached_status = status;
    }

    /// Format the git status as a display string
    pub fn format_status_line(&self) -> String {
        if !self.cached_status.is_repo {
            return String::new();
        }

        let branch = self.cached_status.branch.as_deref().unwrap_or("(detached)");

        if self.cached_status.is_clean() && self.cached_status.is_synced() {
            return format!("{} ✓", branch);
        }

        let mut parts = vec![branch.to_string()];

        // Add ahead/behind indicators
        if self.cached_status.ahead > 0 {
            parts.push(format!("↑{}", self.cached_status.ahead));
        }
        if self.cached_status.behind > 0 {
            parts.push(format!("↓{}", self.cached_status.behind));
        }

        // Add file change indicators
        if self.cached_status.modified > 0 {
            parts.push(format!("*{}", self.cached_status.modified));
        }
        if self.cached_status.staged > 0 {
            parts.push(format!("+{}", self.cached_status.staged));
        }
        if self.cached_status.untracked > 0 {
            parts.push(format!("?{}", self.cached_status.untracked));
        }

        parts.join(" ")
    }

    // Internal helper methods

    fn check_is_repo(&self) -> bool {
        Command::new("git")
            .args(["rev-parse", "--git-dir"])
            .current_dir(&self.repo_path)
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    fn get_branch_name(&self) -> Option<String> {
        let output = Command::new("git")
            .args(["rev-parse", "--abbrev-ref", "HEAD"])
            .current_dir(&self.repo_path)
            .output()
            .ok()?;

        if output.status.success() {
            String::from_utf8(output.stdout)
                .ok()
                .map(|s| s.trim().to_string())
        } else {
            None
        }
    }

    fn get_ahead_behind(&self) -> (usize, usize) {
        // Try to get ahead/behind counts relative to upstream
        let output = Command::new("git")
            .args(["rev-list", "--left-right", "--count", "HEAD...@{u}"])
            .current_dir(&self.repo_path)
            .output();

        if let Ok(output) = output {
            if output.status.success() {
                if let Ok(text) = String::from_utf8(output.stdout) {
                    let parts: Vec<&str> = text.trim().split_whitespace().collect();
                    if parts.len() == 2 {
                        let ahead = parts[0].parse::<usize>().unwrap_or(0);
                        let behind = parts[1].parse::<usize>().unwrap_or(0);
                        return (ahead, behind);
                    }
                }
            }
        }

        // If no upstream or error, return (0, 0)
        (0, 0)
    }

    fn get_file_status(&self) -> (usize, usize, usize) {
        let output = Command::new("git")
            .args(["status", "--porcelain"])
            .current_dir(&self.repo_path)
            .output();

        if let Ok(output) = output {
            if output.status.success() {
                if let Ok(text) = String::from_utf8(output.stdout) {
                    return self.parse_porcelain_status(&text);
                }
            }
        }

        (0, 0, 0)
    }

    fn parse_porcelain_status(&self, porcelain: &str) -> (usize, usize, usize) {
        let mut modified = 0;
        let mut staged = 0;
        let mut untracked = 0;

        for line in porcelain.lines() {
            if line.len() < 3 {
                continue;
            }

            let index_status = line.chars().nth(0).unwrap_or(' ');
            let worktree_status = line.chars().nth(1).unwrap_or(' ');

            // Check for staged changes (index status)
            if index_status != ' ' && index_status != '?' {
                staged += 1;
            }

            // Check for worktree changes
            if worktree_status != ' ' && worktree_status != '?' {
                modified += 1;
            }

            // Check for untracked files
            if index_status == '?' && worktree_status == '?' {
                untracked += 1;
            }
        }

        (modified, staged, untracked)
    }
}

/// Render git status as a styled span for the status bar
pub fn render_git_status(status: &GitStatus) -> Span<'static> {
    if !status.is_repo {
        return Span::raw("");
    }

    let provider = GitStatusProvider {
        repo_path: std::path::PathBuf::new(),
        cached_status: status.clone(),
    };

    let text = provider.format_status_line();

    // Determine color based on status
    let style = if status.is_clean() && status.is_synced() {
        Style::default().fg(Color::Green)
    } else if status.modified > 0 || status.untracked > 0 {
        Style::default().fg(Color::Yellow)
    } else if status.ahead > 0 || status.behind > 0 {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::White)
    };

    Span::styled(text, style)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_git_status_is_clean() {
        let status = GitStatus {
            is_repo: true,
            modified: 0,
            staged: 0,
            untracked: 0,
            ..Default::default()
        };
        assert!(status.is_clean());

        let status = GitStatus {
            is_repo: true,
            modified: 1,
            ..Default::default()
        };
        assert!(!status.is_clean());
    }

    #[test]
    fn test_git_status_is_synced() {
        let status = GitStatus {
            is_repo: true,
            ahead: 0,
            behind: 0,
            ..Default::default()
        };
        assert!(status.is_synced());

        let status = GitStatus {
            is_repo: true,
            ahead: 2,
            ..Default::default()
        };
        assert!(!status.is_synced());
    }

    #[test]
    fn test_parse_porcelain_status() {
        let provider = GitStatusProvider {
            repo_path: std::path::PathBuf::new(),
            cached_status: GitStatus::default(),
        };

        // Test modified file
        let (modified, staged, untracked) = provider.parse_porcelain_status(" M file.txt");
        assert_eq!(modified, 1);
        assert_eq!(staged, 0);
        assert_eq!(untracked, 0);

        // Test staged file
        let (modified, staged, untracked) = provider.parse_porcelain_status("M  file.txt");
        assert_eq!(modified, 0);
        assert_eq!(staged, 1);
        assert_eq!(untracked, 0);

        // Test untracked file
        let (modified, staged, untracked) = provider.parse_porcelain_status("?? file.txt");
        assert_eq!(modified, 0);
        assert_eq!(staged, 0);
        assert_eq!(untracked, 1);

        // Test multiple files
        let porcelain = " M file1.txt\nM  file2.txt\n?? file3.txt";
        let (modified, staged, untracked) = provider.parse_porcelain_status(porcelain);
        assert_eq!(modified, 1);
        assert_eq!(staged, 1);
        assert_eq!(untracked, 1);
    }

    #[test]
    fn test_format_status_line_clean() {
        let provider = GitStatusProvider {
            repo_path: std::path::PathBuf::new(),
            cached_status: GitStatus {
                is_repo: true,
                branch: Some("main".to_string()),
                ..Default::default()
            },
        };

        assert_eq!(provider.format_status_line(), "main ✓");
    }

    #[test]
    fn test_format_status_line_with_changes() {
        let provider = GitStatusProvider {
            repo_path: std::path::PathBuf::new(),
            cached_status: GitStatus {
                is_repo: true,
                branch: Some("main".to_string()),
                ahead: 2,
                behind: 1,
                modified: 3,
                staged: 2,
                untracked: 1,
            },
        };

        assert_eq!(provider.format_status_line(), "main ↑2 ↓1 *3 +2 ?1");
    }

    #[test]
    fn test_format_status_line_no_repo() {
        let provider = GitStatusProvider {
            repo_path: std::path::PathBuf::new(),
            cached_status: GitStatus::default(),
        };

        assert_eq!(provider.format_status_line(), "");
    }
}
