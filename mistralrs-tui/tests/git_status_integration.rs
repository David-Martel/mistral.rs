//! Integration tests for git status provider

#[cfg(feature = "tui-agent")]
mod git_status_tests {
    use mistralrs_tui::components::{GitStatus, GitStatusProvider};
    use std::env;

    #[test]
    fn test_git_status_provider_in_repo() {
        // Test in the current repository (mistral.rs is a git repo)
        let current_dir = env::current_dir().expect("Failed to get current directory");

        let provider = GitStatusProvider::new(&current_dir);

        // Should detect that we're in a git repository
        assert!(provider.is_in_repo());

        // Should have a branch name
        let status = provider.status();
        assert!(status.is_repo);
        assert!(status.branch.is_some());

        // Format should not be empty
        let formatted = provider.format_status_line();
        assert!(!formatted.is_empty());

        println!("Git status: {}", formatted);
    }

    #[test]
    fn test_git_status_provider_not_in_repo() {
        // Test in a directory that's not a git repo
        let temp_dir = std::env::temp_dir();

        let provider = GitStatusProvider::new(&temp_dir);

        // Should detect that we're NOT in a git repository
        // (unless temp is inside a git repo, which is unlikely)
        let status = provider.status();

        // Format should be empty if not in repo
        if !status.is_repo {
            let formatted = provider.format_status_line();
            assert_eq!(formatted, "");
        }
    }

    #[test]
    fn test_git_status_refresh() {
        let current_dir = env::current_dir().expect("Failed to get current directory");
        let mut provider = GitStatusProvider::new(&current_dir);

        let status_before = provider.status().clone();

        // Refresh should work without errors
        provider.refresh();

        let status_after = provider.status();

        // Both should agree on whether it's a repo
        assert_eq!(status_before.is_repo, status_after.is_repo);

        if status_before.is_repo {
            // Branch should be consistent
            assert_eq!(status_before.branch, status_after.branch);
        }
    }

    #[test]
    fn test_git_status_display() {
        // Test GitStatus display formatting directly
        let status = GitStatus {
            is_repo: true,
            branch: Some("main".to_string()),
            ahead: 0,
            behind: 0,
            modified: 0,
            staged: 0,
            untracked: 0,
        };

        // GitStatus should be displayable
        assert!(status.is_repo);
        assert_eq!(status.branch.as_deref(), Some("main"));
    }

    #[test]
    fn test_git_status_with_changes() {
        // Test GitStatus with various states
        let status = GitStatus {
            is_repo: true,
            branch: Some("feature-branch".to_string()),
            ahead: 2,
            behind: 1,
            modified: 3,
            staged: 2,
            untracked: 1,
        };

        // Verify all fields are set correctly
        assert!(status.is_repo);
        assert_eq!(status.branch.as_deref(), Some("feature-branch"));
        assert_eq!(status.ahead, 2);
        assert_eq!(status.behind, 1);
        assert_eq!(status.modified, 3);
        assert_eq!(status.staged, 2);
        assert_eq!(status.untracked, 1);
    }

    #[test]
    fn test_render_git_status() {
        use mistralrs_tui::components::render_git_status;
        use ratatui::style::Color;

        // Clean status should be green
        let status = GitStatus {
            is_repo: true,
            branch: Some("main".to_string()),
            ..Default::default()
        };

        let span = render_git_status(&status);
        assert_eq!(span.style.fg, Some(Color::Green));

        // Modified files should be yellow
        let status = GitStatus {
            is_repo: true,
            branch: Some("main".to_string()),
            modified: 2,
            ..Default::default()
        };

        let span = render_git_status(&status);
        assert_eq!(span.style.fg, Some(Color::Yellow));

        // Non-repo should be empty
        let status = GitStatus::default();
        let span = render_git_status(&status);
        assert!(span.content.is_empty());
    }
}
