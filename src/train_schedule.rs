//! Train Schedule Module
//!
//! Implements predictable PR bundling schedule visibility for agents.
//! PRs are bundled at 10-minute intervals (:00, :10, :20, :30, :40, :50)
//! but only when clambake land is manually triggered at/after departure time.

use chrono::{DateTime, Local, Timelike};
use std::process::Command;

#[derive(Debug, Clone)]
pub struct TrainSchedule {
    /// Next bundling opportunity
    pub next_departure: DateTime<Local>,
    /// Minutes until next departure
    pub minutes_until_departure: i64,
    /// Current schedule status
    pub status: ScheduleStatus,
}

#[derive(Debug, Clone)]
pub enum ScheduleStatus {
    /// Agents can still add work before next departure
    Boarding,
    /// At/past departure time, ready for bundling
    Departing,
    /// No work queued, waiting for next cycle
    Waiting,
}

#[derive(Debug, Clone)]
pub struct QueuedBranch {
    pub branch_name: String,
    pub issue_number: u64,
    pub description: String,
}

impl TrainSchedule {
    /// Calculate the next train schedule based on current time
    pub fn calculate_next_schedule() -> Self {
        let now = Local::now();
        let current_minute = now.minute();

        // Calculate next 10-minute mark
        let next_departure_minute = ((current_minute / 10) + 1) * 10;
        let minutes_to_add = if next_departure_minute >= 60 {
            60 - current_minute // Roll to next hour
        } else {
            next_departure_minute - current_minute
        };

        let next_departure = now.with_second(0).unwrap().with_nanosecond(0).unwrap()
            + chrono::Duration::minutes(minutes_to_add as i64);

        let minutes_until = (next_departure - now).num_minutes();

        let status = if minutes_until <= 0 {
            ScheduleStatus::Departing
        } else if minutes_until <= 3 {
            ScheduleStatus::Boarding
        } else {
            ScheduleStatus::Waiting
        };

        TrainSchedule {
            next_departure,
            minutes_until_departure: minutes_until,
            status,
        }
    }

    /// Check if we're at or past a departure time
    pub fn is_departure_time() -> bool {
        let schedule = Self::calculate_next_schedule();
        matches!(schedule.status, ScheduleStatus::Departing)
    }

    /// Get all agent branches that are ready for bundling
    pub async fn get_queued_branches() -> Result<Vec<QueuedBranch>, Box<dyn std::error::Error>> {
        let mut queued_branches = Vec::new();

        // Check both local and remote agent branches
        let mut all_branches = std::collections::HashSet::new();

        // Get local agent branches
        let local_output = Command::new("git")
            .args(["branch", "--list", "agent*"])
            .output()?;

        if local_output.status.success() {
            let local_branches_str = String::from_utf8_lossy(&local_output.stdout);
            for line in local_branches_str.lines() {
                let branch = line.trim().trim_start_matches("* ").trim();
                if !branch.is_empty() {
                    all_branches.insert(branch.to_string());
                }
            }
        }

        // Get remote agent branches
        let remote_output = Command::new("git")
            .args(["branch", "-r", "--list", "origin/agent*"])
            .output()?;

        if remote_output.status.success() {
            let remote_branches_str = String::from_utf8_lossy(&remote_output.stdout);
            for line in remote_branches_str.lines() {
                let branch = line.trim().strip_prefix("origin/").unwrap_or(line.trim());
                if !branch.is_empty() {
                    all_branches.insert(branch.to_string());
                }
            }
        }

        // Check each unique branch for completed work
        for branch in all_branches {
            // Parse agent001/123 format
            if let Some((agent_part, issue_number_str)) = branch.split_once('/') {
                if agent_part.starts_with("agent") {
                    if let Ok(issue_number) = issue_number_str.parse::<u64>() {
                        // Check if this branch has work ready for bundling (handles both local and remote)
                        if Self::branch_has_completed_work(&branch).await? {
                            let description = Self::get_branch_description(issue_number)
                                .await
                                .unwrap_or_else(|_| "Work completed".to_string());

                            queued_branches.push(QueuedBranch {
                                branch_name: branch.to_string(),
                                issue_number,
                                description,
                            });
                        }
                    }
                }
            }
        }

        Ok(queued_branches)
    }

    /// Check if a branch has completed work (local commits or route:review label)
    async fn branch_has_completed_work(
        branch_name: &str,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        // Parse issue number from branch name (agent001/123 -> 123)
        let issue_number = if let Some((_, issue_number_str)) = branch_name.split_once('/') {
            issue_number_str.parse::<u64>().unwrap_or(0)
        } else {
            return Ok(false);
        };

        if issue_number == 0 {
            return Ok(false);
        }

        // First check if the issue has route:review label (work already landed)
        let output = Command::new("gh")
            .args([
                "issue",
                "view",
                &issue_number.to_string(),
                "--json",
                "labels,state",
            ])
            .output()?;

        if output.status.success() {
            let json_str = String::from_utf8_lossy(&output.stdout);
            let is_open = json_str.contains("\"state\":\"OPEN\"");
            let has_route_review = json_str.contains("\"name\":\"route:review\"");

            // If issue has route:review label, it's definitely ready for bundling
            if is_open && has_route_review {
                return Ok(true);
            }
        }

        // Check if local branch exists and has commits ahead of main
        let local_commits_output = Command::new("git")
            .args(["rev-list", "--count", &format!("main..{branch_name}")])
            .output()?;

        if local_commits_output.status.success() {
            let commits_ahead_str = String::from_utf8_lossy(&local_commits_output.stdout)
                .trim()
                .to_string();
            let commits_ahead: u32 = commits_ahead_str.parse().unwrap_or(0);

            // If there are local commits ahead, this branch has work to bundle
            if commits_ahead > 0 {
                return Ok(true);
            }
        }

        // Check if remote branch has commits ahead of main (fallback)
        let remote_commits_output = Command::new("git")
            .args([
                "rev-list",
                "--count",
                &format!("main..origin/{branch_name}"),
            ])
            .output()?;

        if remote_commits_output.status.success() {
            let commits_ahead_str = String::from_utf8_lossy(&remote_commits_output.stdout)
                .trim()
                .to_string();
            let commits_ahead: u32 = commits_ahead_str.parse().unwrap_or(0);

            return Ok(commits_ahead > 0);
        }

        Ok(false)
    }

    /// Get a description for the branch work from the issue
    async fn get_branch_description(
        issue_number: u64,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let output = Command::new("gh")
            .args([
                "issue",
                "view",
                &issue_number.to_string(),
                "--json",
                "title",
            ])
            .output()?;

        if output.status.success() {
            let json_str = String::from_utf8_lossy(&output.stdout);
            // Simple JSON parsing for title field
            if let Some(start) = json_str.find("\"title\":\"") {
                let start = start + 9; // length of "title":"
                if let Some(end) = json_str[start..].find("\"}") {
                    let title = &json_str[start..start + end];
                    return Ok(title.to_string());
                }
            }
        }

        Ok(format!("Issue #{issue_number}"))
    }

    /// Format the schedule for display
    pub fn format_schedule_display(&self, queued_branches: &[QueuedBranch]) -> String {
        let mut output = String::new();

        output.push_str("🚄 PR BUNDLING SCHEDULE:\n");
        output.push_str("─────────────────────\n");

        // Status line with time
        let time_str = self.next_departure.format("%H:%M").to_string();
        match self.status {
            ScheduleStatus::Departing => {
                output.push_str(&format!("🟢 Next train: {time_str} (READY TO DEPART)\n"));
            }
            ScheduleStatus::Boarding => {
                output.push_str(&format!(
                    "🟡 Next train: {} (in {} min)\n",
                    time_str, self.minutes_until_departure
                ));
            }
            ScheduleStatus::Waiting => {
                output.push_str(&format!(
                    "🔵 Next train: {} (in {} min)\n",
                    time_str, self.minutes_until_departure
                ));
            }
        }

        // Queued branches
        if queued_branches.is_empty() {
            output.push_str("📦 Queued branches: None\n");
        } else {
            output.push_str(&format!("📦 Queued branches: {}\n", queued_branches.len()));
            for branch in queued_branches.iter().take(5) {
                // Show max 5
                output.push_str(&format!(
                    "   • {} ({})\n",
                    branch.branch_name, branch.description
                ));
            }
            if queued_branches.len() > 5 {
                output.push_str(&format!(
                    "   • ... and {} more\n",
                    queued_branches.len() - 5
                ));
            }
        }

        output.push_str("⏰ Schedule: :00, :10, :20, :30, :40, :50\n");

        if matches!(self.status, ScheduleStatus::Departing) && !queued_branches.is_empty() {
            output.push_str("\n💡 Run 'clambake land' to bundle queued branches into PR\n");
        }

        output
    }

    /// Get branches that are past their departure time by more than 10 minutes (overdue)
    pub async fn get_overdue_branches() -> Result<Vec<QueuedBranch>, Box<dyn std::error::Error>> {
        let mut overdue_branches = Vec::new();

        // Get all agent branches
        let output = Command::new("git")
            .args(["branch", "-r", "--list", "origin/agent*"])
            .output()?;

        if !output.status.success() {
            return Ok(overdue_branches);
        }

        let branches_str = String::from_utf8_lossy(&output.stdout);

        for line in branches_str.lines() {
            let branch = line.trim().strip_prefix("origin/").unwrap_or(line.trim());

            // Parse agent001/123 format
            if let Some((agent_part, issue_number_str)) = branch.split_once('/') {
                if agent_part.starts_with("agent") {
                    if let Ok(issue_number) = issue_number_str.parse::<u64>() {
                        // Check if this branch has work and is overdue
                        if Self::branch_has_completed_work(branch).await? {
                            // Get the last commit time on this branch
                            if let Ok(minutes_since_commit) =
                                Self::get_minutes_since_last_commit(branch).await
                            {
                                // Calculate expected departure time based on commit time
                                let departure_delay =
                                    Self::calculate_departure_delay(minutes_since_commit);

                                // Branch is overdue if it's been more than 10 minutes past expected departure
                                if departure_delay > 10 {
                                    let description = Self::get_branch_description(issue_number)
                                        .await
                                        .unwrap_or_else(|_| "Work completed".to_string());

                                    overdue_branches.push(QueuedBranch {
                                        branch_name: branch.to_string(),
                                        issue_number,
                                        description: format!(
                                            "{description} ({departure_delay} min overdue)"
                                        ),
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(overdue_branches)
    }

    /// Get minutes since last commit on a branch
    async fn get_minutes_since_last_commit(
        branch_name: &str,
    ) -> Result<i64, Box<dyn std::error::Error>> {
        let output = Command::new("git")
            .args([
                "log",
                "-1",
                "--format=%ct",
                &format!("origin/{branch_name}"),
            ])
            .output()?;

        if !output.status.success() {
            return Err("Failed to get commit time".into());
        }

        let timestamp_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let timestamp: i64 = timestamp_str.parse()?;

        let commit_time = DateTime::from_timestamp(timestamp, 0)
            .ok_or("Invalid timestamp")?
            .with_timezone(&Local);
        let now = Local::now();

        Ok((now - commit_time).num_minutes())
    }

    /// Calculate how many minutes past expected departure time
    fn calculate_departure_delay(minutes_since_commit: i64) -> i64 {
        // Find the next 10-minute departure time after the commit
        let expected_departure_minutes = ((minutes_since_commit / 10) + 1) * 10;

        // How many minutes past that departure time are we now?
        std::cmp::max(0, minutes_since_commit - expected_departure_minutes)
    }
}
