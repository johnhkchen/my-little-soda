/// GitHub label management functionality
///
/// Contains all logic for creating and managing GitHub repository labels
/// that are required for the My Little Soda workflow system.

use crate::github::client::GitHubClient;
use anyhow::{anyhow, Result};
use std::io::Write;

use super::core::{InitCommand, LabelSpec};
use super::validation;

/// Setup required GitHub labels for the repository
pub async fn setup_labels(init_command: &InitCommand) -> Result<()> {
    let labels = get_required_labels();

    if init_command.dry_run {
        println!("Would create {} labels:", labels.len());
        for label in &labels {
            println!(
                "  🏷️  {} (#{}) - {}",
                label.name, label.color, label.description
            );
        }
        return Ok(());
    }

    // Check if this is a fresh project - skip label creation
    let is_fresh_project = validation::detect_fresh_project(init_command).await;
    if is_fresh_project {
        println!("⏭️  Skipping GitHub label creation for fresh project");
        println!("   Labels will be created after GitHub repository setup");
        return Ok(());
    }

    let github_client = GitHubClient::with_verbose(init_command.verbose)
        .map_err(|e| anyhow!("Failed to create GitHub client: {}", e))?;

    let octocrab = github_client.issues.octocrab();

    for label in &labels {
        print!("🏷️  Creating label '{}' ", label.name);
        std::io::stdout().flush().unwrap();

        match octocrab
            .issues(github_client.owner(), github_client.repo())
            .create_label(&label.name, &label.color, &label.description)
            .await
        {
            Ok(_) => println!("✅"),
            Err(octocrab::Error::GitHub { source, .. })
                if source.message.contains("already_exists") =>
            {
                println!("⚠️ (already exists)");
            }
            Err(e) => {
                return Err(anyhow!("Failed to create label '{}': {}", label.name, e));
            }
        }
    }

    Ok(())
}

/// Get the complete list of required labels for My Little Soda
pub fn get_required_labels() -> Vec<LabelSpec> {
    vec![
        // Core routing labels
        LabelSpec {
            name: "route:ready".to_string(),
            color: "0052cc".to_string(),
            description: "Available for agent assignment".to_string(),
        },
        LabelSpec {
            name: "route:ready_to_merge".to_string(),
            color: "5319e7".to_string(),
            description: "Completed work ready for merge".to_string(),
        },
        LabelSpec {
            name: "route:unblocker".to_string(),
            color: "d73a4a".to_string(),
            description: "Critical system issues blocking other work".to_string(),
        },
        LabelSpec {
            name: "route:review".to_string(),
            color: "fbca04".to_string(),
            description: "Under review".to_string(),
        },
        LabelSpec {
            name: "route:human-only".to_string(),
            color: "7057ff".to_string(),
            description: "Requires human attention".to_string(),
        },
        // Priority labels
        LabelSpec {
            name: "route:priority-low".to_string(),
            color: "c5def5".to_string(),
            description: "Low priority task (Priority: 1)".to_string(),
        },
        LabelSpec {
            name: "route:priority-medium".to_string(),
            color: "1d76db".to_string(),
            description: "Medium priority task (Priority: 2)".to_string(),
        },
        LabelSpec {
            name: "route:priority-high".to_string(),
            color: "b60205".to_string(),
            description: "High priority task (Priority: 3)".to_string(),
        },
        LabelSpec {
            name: "route:priority-very-high".to_string(),
            color: "ee0701".to_string(),
            description: "Very high priority task (Priority: 4)".to_string(),
        },
        // Additional operational labels
        LabelSpec {
            name: "code-review-feedback".to_string(),
            color: "e99695".to_string(),
            description: "Issues created from code review feedback".to_string(),
        },
        LabelSpec {
            name: "supertask-decomposition".to_string(),
            color: "bfdadc".to_string(),
            description: "Task broken down from larger work".to_string(),
        },
        LabelSpec {
            name: "code-quality".to_string(),
            color: "d4c5f9".to_string(),
            description: "Code quality improvements, refactoring, and technical debt reduction"
                .to_string(),
        },
    ]
}