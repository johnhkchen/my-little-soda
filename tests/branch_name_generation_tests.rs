use my_little_soda::agents::routing::assignment::AssignmentOperations;

#[cfg(test)]
mod branch_name_tests {
    use super::*;

    #[test]
    fn test_short_title_unchanged() {
        let ops = AssignmentOperations::new();
        let branch_name = ops.generate_branch_name("agent001", 123, "Short title");
        assert_eq!(branch_name, "agent001/123-short-title");
    }

    #[test]
    fn test_truncation_preserves_words() {
        let ops = AssignmentOperations::new();
        // This title would be exactly 30 chars when joined: "fix-doctor-json-mode-currently"
        let branch_name = ops.generate_branch_name("agent001", 431, "Fix doctor JSON mode currently non-functional");
        // Should truncate at word boundary, not mid-word
        assert!(branch_name.contains("agent001/431-"));
        assert!(!branch_name.ends_with("--")); // Should not have double dash
        assert!(!branch_name.contains("non-functional")); // Should be truncated
    }

    #[test]
    fn test_no_double_dash_bug() {
        let ops = AssignmentOperations::new();
        // Test various titles that previously caused double-dash bugs
        let test_cases = vec![
            "FEATURE Fix doctor JSON mode currently non-functional",
            "INFRASTRUCTURE Improve branch creation reliability using git2 local operations",
            "BUG Fix authentication timeout in production environment",
        ];
        
        for title in test_cases {
            let branch_name = ops.generate_branch_name("agent001", 432, title);
            assert!(!branch_name.ends_with("--"), "Branch name should not end with double dash: {}", branch_name);
            assert!(!branch_name.contains("---"), "Branch name should not contain triple dash: {}", branch_name);
        }
    }

    #[test]
    fn test_sanitization() {
        let ops = AssignmentOperations::new();
        let branch_name = ops.generate_branch_name("agent001", 123, "Fix [URGENT] issue with @#$%^&*() special chars!");
        // Truncated at word boundary due to 30-char limit
        assert_eq!(branch_name, "agent001/123-fix-urgent-issue-with-special");
    }

    #[test]
    fn test_multiple_spaces_and_dashes() {
        let ops = AssignmentOperations::new();
        let branch_name = ops.generate_branch_name("agent001", 123, "Fix   multiple---spaces  and-dashes");
        assert_eq!(branch_name, "agent001/123-fix-multiple-spaces-and-dashes");
    }

    #[test]
    fn test_empty_title() {
        let ops = AssignmentOperations::new();
        let branch_name = ops.generate_branch_name("agent001", 123, "");
        assert_eq!(branch_name, "agent001/123-");
    }

    #[test]
    fn test_long_single_word() {
        let ops = AssignmentOperations::new();
        // Single word longer than 30 chars should be truncated at char boundary
        let long_word = "supercalifragilisticexpialidocious";
        let branch_name = ops.generate_branch_name("agent001", 123, long_word);
        let slug = branch_name.split('-').skip(1).collect::<Vec<_>>().join("-");
        assert!(slug.len() <= 30);
        // Should start with the word but be truncated at 30 chars
        assert!(slug.starts_with("supercalifragilisticexpialidoc"));
    }

    #[test]
    fn test_exactly_30_chars() {
        let ops = AssignmentOperations::new();
        // Create a title that when slugified is exactly 30 chars
        let title_30 = "this-is-exactly-thirty-charss"; // 30 chars
        let branch_name = ops.generate_branch_name("agent001", 123, title_30);
        let slug = branch_name.split('-').skip(1).collect::<Vec<_>>().join("-");
        // The actual slug will be 29 chars because we have "this-is-exactly-thirty-charss" 
        // which becomes "this-is-exactly-thirty-charss" (29 chars, not 30)
        assert!(slug.len() <= 30);
        assert_eq!(slug, "this-is-exactly-thirty-charss");
    }

    #[test]
    fn test_mixed_case_normalization() {
        let ops = AssignmentOperations::new();
        let branch_name = ops.generate_branch_name("agent001", 123, "FiX BUG In AUTHENTICATION Module");
        // Truncated at word boundary due to 30-char limit
        assert_eq!(branch_name, "agent001/123-fix-bug-in-authentication");
    }

    #[test]
    fn test_unicode_characters_filtered() {
        let ops = AssignmentOperations::new();
        let branch_name = ops.generate_branch_name("agent001", 123, "Fix émoji 🐛 and ñice ünïcödë");
        // Unicode chars should be filtered out, only alphanumeric remain
        assert_eq!(branch_name, "agent001/123-fix-moji-and-ice-ncd");
    }

    #[test]
    fn test_preserves_meaningful_parts() {
        let ops = AssignmentOperations::new();
        // Test that truncation preserves the most meaningful parts
        let branch_name = ops.generate_branch_name("agent001", 123, "INFRASTRUCTURE Improve branch creation reliability using git2 operations");
        let slug = branch_name.split('-').skip(1).collect::<Vec<_>>().join("-");
        
        // Should start with the important words
        assert!(slug.starts_with("infrastructure"));
        assert!(slug.contains("improve"));
        assert!(slug.contains("branch"));
        // But may not include all words due to truncation
        assert!(slug.len() <= 30);
    }

}