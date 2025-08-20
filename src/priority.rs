use std::fmt;

/// Priority levels for GitHub issues in the clambake routing system
/// Higher values = higher priority in the queue
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    /// No priority label (0)
    Normal = 0,
    /// route:priority-low (1)
    Low = 1,
    /// route:priority-medium (2)
    Medium = 2,
    /// route:priority-high (3)
    High = 3,
    /// route:land - merge-ready work (100)
    MergeReady = 100,
    /// route:unblocker - critical system issues (200)
    Unblocker = 200,
}

impl Priority {
    /// Determine priority from GitHub issue labels
    pub fn from_labels(labels: &[impl AsRef<str>]) -> Self {
        let mut highest_priority = Priority::Normal;
        
        for label in labels {
            let priority = match label.as_ref() {
                "route:unblocker" => Priority::Unblocker,
                "route:land" => Priority::MergeReady,
                "route:priority-high" => Priority::High,
                "route:priority-medium" => Priority::Medium,
                "route:priority-low" => Priority::Low,
                _ => continue,
            };
            
            if priority > highest_priority {
                highest_priority = priority;
            }
        }
        
        highest_priority
    }

    /// Get the numeric priority value
    pub fn value(self) -> u32 {
        self as u32
    }
}

impl fmt::Display for Priority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Priority::Unblocker => "UNBLOCKER",
            Priority::MergeReady => "MERGE READY",
            Priority::High => "HIGH",
            Priority::Medium => "MEDIUM",
            Priority::Low => "LOW",
            Priority::Normal => "NORMAL",
        };
        write!(f, "{}", label)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_priority_from_labels() {
        // Test unblocker (highest)
        assert_eq!(Priority::from_labels(&["route:unblocker"]), Priority::Unblocker);
        
        // Test land (second highest)
        assert_eq!(Priority::from_labels(&["route:land"]), Priority::MergeReady);
        
        // Test standard priorities
        assert_eq!(Priority::from_labels(&["route:priority-high"]), Priority::High);
        assert_eq!(Priority::from_labels(&["route:priority-medium"]), Priority::Medium);
        assert_eq!(Priority::from_labels(&["route:priority-low"]), Priority::Low);
        
        // Test normal (no priority labels)
        assert_eq!(Priority::from_labels(&["route:ready"]), Priority::Normal);
        assert_eq!(Priority::from_labels(&[] as &[&str]), Priority::Normal);
        
        // Test precedence (unblocker wins over high)
        assert_eq!(Priority::from_labels(&["route:priority-high", "route:unblocker"]), Priority::Unblocker);
    }

    #[test]
    fn test_priority_values() {
        assert_eq!(Priority::Normal.value(), 0);
        assert_eq!(Priority::Low.value(), 1);
        assert_eq!(Priority::Medium.value(), 2);
        assert_eq!(Priority::High.value(), 3);
        assert_eq!(Priority::MergeReady.value(), 100);
        assert_eq!(Priority::Unblocker.value(), 200);
    }

    #[test]
    fn test_priority_ordering() {
        assert!(Priority::Unblocker > Priority::MergeReady);
        assert!(Priority::MergeReady > Priority::High);
        assert!(Priority::High > Priority::Medium);
        assert!(Priority::Medium > Priority::Low);
        assert!(Priority::Low > Priority::Normal);
    }

    #[test]
    fn test_priority_display() {
        assert_eq!(Priority::Unblocker.to_string(), "UNBLOCKER");
        assert_eq!(Priority::MergeReady.to_string(), "MERGE READY");
        assert_eq!(Priority::High.to_string(), "HIGH");
        assert_eq!(Priority::Medium.to_string(), "MEDIUM");
        assert_eq!(Priority::Low.to_string(), "LOW");
        assert_eq!(Priority::Normal.to_string(), "NORMAL");
    }
}