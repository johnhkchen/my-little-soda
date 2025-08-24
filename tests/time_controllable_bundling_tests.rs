//! Time-controllable bundling tests
//!
//! These tests use tokio::time controls to make bundling tests deterministic
//! and fast by eliminating real-time waits in 10-minute bundling cycles.
//!
//! Note: These tests validate the bundling timing logic patterns and deterministic
//! behavior rather than controlling real time, since the current implementation
//! uses system time directly.

use chrono::Local;
use my_little_soda::bundling::types::BundleWindow;
use my_little_soda::train_schedule::{ScheduleStatus, TrainSchedule};
use tokio::time::Duration;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_bundling_schedule_timing_logic() {
        // Test the schedule calculation logic works correctly
        // Note: This uses real system time but validates the logic patterns

        let schedule = TrainSchedule::calculate_next_schedule();

        // Schedule should be valid
        assert!(
            schedule.next_departure > Local::now(),
            "Next departure should be in the future"
        );

        // Minutes until departure should be reasonable (0-10 minutes)
        assert!(
            schedule.minutes_until_departure >= 0,
            "Minutes until departure should be non-negative"
        );
        assert!(
            schedule.minutes_until_departure <= 10,
            "Minutes until departure should be at most 10"
        );

        // Status should match the minutes until departure
        match schedule.status {
            ScheduleStatus::Waiting => {
                assert!(
                    schedule.minutes_until_departure > 3,
                    "Waiting status should have > 3 minutes"
                );
            }
            ScheduleStatus::Boarding => {
                assert!(
                    schedule.minutes_until_departure <= 3 && schedule.minutes_until_departure > 0,
                    "Boarding status should have 1-3 minutes"
                );
            }
            ScheduleStatus::Departing => {
                assert!(
                    schedule.minutes_until_departure <= 0,
                    "Departing status should have <= 0 minutes"
                );
            }
        }
    }

    #[tokio::test]
    async fn test_bundle_window_deterministic_naming() {
        // Test bundle window naming is deterministic for same input
        let current_window = BundleWindow::current();

        // Test bundle branch naming with deterministic issues
        let issues = vec![123, 456, 789];
        let branch_name = current_window.bundle_branch_name(&issues);

        // Branch name should include sorted issues and timestamp
        assert!(branch_name.contains("bundle/"));
        assert!(branch_name.contains("issues_123_456_789"));

        // Test with same issues in different order - should produce same name
        let issues_reordered = vec![789, 123, 456];
        let branch_name_2 = current_window.bundle_branch_name(&issues_reordered);

        assert_eq!(
            branch_name, branch_name_2,
            "Bundle names should be deterministic regardless of input order"
        );

        // Branch name should contain the window timestamp
        assert!(branch_name.starts_with("bundle/"));
        let timestamp_part = branch_name.split("__issues_").next().unwrap();
        assert!(
            timestamp_part.contains("bundle/"),
            "Should contain bundle prefix"
        );
    }

    #[tokio::test]
    async fn test_bundle_window_consistency() {
        // Test that bundle windows created at the same time are consistent
        let window1 = BundleWindow::current();

        // Small delay to ensure we're still in same window
        tokio::time::sleep(Duration::from_millis(10)).await;
        let window2 = BundleWindow::current();

        // Windows should be very close in time (within same 10-minute window)
        let time_diff = (window2.start - window1.start).num_seconds().abs();
        assert!(
            time_diff < 60,
            "Bundle windows should be consistent within short time period"
        );

        // Test that different issues produce different branch names
        let issues1 = vec![100, 200];
        let issues2 = vec![300, 400];

        let branch1 = window1.bundle_branch_name(&issues1);
        let branch2 = window1.bundle_branch_name(&issues2);

        assert_ne!(
            branch1, branch2,
            "Different issues should produce different branch names"
        );
        assert!(
            branch1.contains("issues_100_200"),
            "Branch should contain first issue set"
        );
        assert!(
            branch2.contains("issues_300_400"),
            "Branch should contain second issue set"
        );
    }

    #[tokio::test]
    async fn test_departure_time_detection_logic() {
        // Test departure time detection logic consistency
        let is_departure_1 = TrainSchedule::is_departure_time();
        let schedule_1 = TrainSchedule::calculate_next_schedule();

        // is_departure_time() should match the schedule status
        let expected_departure = matches!(schedule_1.status, ScheduleStatus::Departing);
        assert_eq!(
            is_departure_1, expected_departure,
            "is_departure_time() should match schedule status"
        );

        // Small delay and test again for consistency
        tokio::time::sleep(Duration::from_millis(100)).await;

        let is_departure_2 = TrainSchedule::is_departure_time();
        let schedule_2 = TrainSchedule::calculate_next_schedule();

        // Should still be consistent within short time window
        let expected_departure_2 = matches!(schedule_2.status, ScheduleStatus::Departing);
        assert_eq!(
            is_departure_2, expected_departure_2,
            "Detection should remain consistent over short time"
        );
    }

    #[tokio::test]
    async fn test_schedule_status_transitions_logic() {
        // Test that schedule status logic is consistent
        let schedule = TrainSchedule::calculate_next_schedule();

        // Verify status matches minutes_until_departure
        match schedule.status {
            ScheduleStatus::Waiting => {
                assert!(
                    schedule.minutes_until_departure > 3,
                    "Waiting status should have > 3 minutes until departure, got {}",
                    schedule.minutes_until_departure
                );
            }
            ScheduleStatus::Boarding => {
                assert!(
                    schedule.minutes_until_departure <= 3 && schedule.minutes_until_departure > 0,
                    "Boarding status should have 1-3 minutes until departure, got {}",
                    schedule.minutes_until_departure
                );
            }
            ScheduleStatus::Departing => {
                assert!(
                    schedule.minutes_until_departure <= 0,
                    "Departing status should have <= 0 minutes until departure, got {}",
                    schedule.minutes_until_departure
                );
            }
        }

        // Test that next_departure is always in the future (unless departing)
        if !matches!(schedule.status, ScheduleStatus::Departing) {
            assert!(
                schedule.next_departure > Local::now(),
                "Next departure should be in the future for non-departing status"
            );
        }
    }

    #[tokio::test]
    async fn test_bundle_name_uniqueness() {
        // Test that bundle names are unique for different issue combinations
        let window = BundleWindow::current();

        let mut bundle_names = std::collections::HashSet::new();

        // Generate multiple bundle names with different issue combinations
        for i in 0..10 {
            let issues = vec![100 + i, 200 + i];
            let bundle_name = window.bundle_branch_name(&issues);

            // Each bundle name should be unique
            assert!(
                bundle_names.insert(bundle_name.clone()),
                "Bundle name should be unique: {}",
                bundle_name
            );

            // Name should contain the expected issues
            assert!(
                bundle_name.contains(&format!("issues_{}_{}", 100 + i, 200 + i)),
                "Bundle name should contain issue numbers: {}",
                bundle_name
            );
        }

        // Should have 10 unique bundle names
        assert_eq!(
            bundle_names.len(),
            10,
            "Should generate 10 unique bundle names"
        );

        // All names should start with bundle prefix
        for name in &bundle_names {
            assert!(
                name.starts_with("bundle/"),
                "All names should start with 'bundle/': {}",
                name
            );
        }
    }
}
