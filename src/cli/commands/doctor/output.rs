use super::types::{DiagnosticResult, DiagnosticStatus, DiagnosticSummary, DiagnosticReport, SystemReadiness, ReadinessStatus, ActionableRecommendation, RecommendationPriority};
use crate::cli::DoctorFormat;
use anyhow::Result;
use std::collections::HashMap;

/// Output and reporting functionality for diagnostics
pub struct DiagnosticOutput {
    format: DoctorFormat,
    verbose: bool,
}

impl DiagnosticOutput {
    pub fn new(format: DoctorFormat, verbose: bool) -> Self {
        Self { format, verbose }
    }

    fn is_verbose(&self) -> bool {
        self.verbose && !matches!(self.format, DoctorFormat::Json)
    }

    pub fn calculate_summary(&self, checks: &HashMap<String, DiagnosticResult>) -> DiagnosticSummary {
        let total_checks = checks.len();
        let mut passed = 0;
        let mut failed = 0;
        let mut warnings = 0;
        let mut info = 0;

        for result in checks.values() {
            match result.status {
                DiagnosticStatus::Pass => passed += 1,
                DiagnosticStatus::Fail => failed += 1,
                DiagnosticStatus::Warning => warnings += 1,
                DiagnosticStatus::Info => info += 1,
            }
        }

        DiagnosticSummary {
            total_checks,
            passed,
            failed,
            warnings,
            info,
        }
    }

    pub fn output_report(&self, report: &DiagnosticReport) -> Result<()> {
        match self.format {
            DoctorFormat::Json => {
                println!("{}", serde_json::to_string_pretty(report)?);
            }
            DoctorFormat::Text => {
                self.output_text_report(report)?;
            }
        }
        Ok(())
    }

    fn output_text_report(&self, report: &DiagnosticReport) -> Result<()> {
        // Header
        println!("🩺 MY LITTLE SODA DOCTOR - System Diagnostics");
        println!("=============================================");
        println!();

        // Summary
        println!("📊 DIAGNOSTIC SUMMARY:");
        println!("─────────────────────");
        println!("Total checks: {}", report.summary.total_checks);
        if report.summary.passed > 0 {
            println!("✅ Passed: {}", report.summary.passed);
        }
        if report.summary.failed > 0 {
            println!("❌ Failed: {}", report.summary.failed);
        }
        if report.summary.warnings > 0 {
            println!("⚠️  Warnings: {}", report.summary.warnings);
        }
        if report.summary.info > 0 {
            println!("ℹ️  Info: {}", report.summary.info);
        }
        println!();

        // Detailed results
        println!("🔍 DETAILED RESULTS:");
        println!("──────────────────");

        // Sort checks for consistent output
        let mut sorted_checks: Vec<_> = report.checks.iter().collect();
        sorted_checks.sort_by_key(|(name, _)| *name);

        for (name, result) in sorted_checks {
            let status_icon = match result.status {
                DiagnosticStatus::Pass => "✅",
                DiagnosticStatus::Fail => "❌",
                DiagnosticStatus::Warning => "⚠️",
                DiagnosticStatus::Info => "ℹ️",
            };

            println!("{} {}: {}", status_icon, name, result.message);

            if self.is_verbose()
                || matches!(
                    result.status,
                    DiagnosticStatus::Fail | DiagnosticStatus::Warning
                )
            {
                if let Some(details) = &result.details {
                    println!("   Details: {}", details);
                }
                if let Some(suggestion) = &result.suggestion {
                    println!("   Suggestion: {}", suggestion);
                }
            }
            println!();
        }

        // Overall status
        if report.summary.failed == 0 {
            if report.summary.warnings > 0 {
                println!(
                    "⚠️  System is functional but has {} warning(s) that should be addressed.",
                    report.summary.warnings
                );
            } else {
                println!("✅ System is healthy and ready for use!");
            }
        } else {
            println!(
                "❌ System has {} critical issue(s) that must be resolved.",
                report.summary.failed
            );
        }

        // System Readiness Score
        self.output_system_readiness(&report.readiness);

        // Actionable Recommendations
        self.output_actionable_recommendations(&report.recommendations);

        Ok(())
    }

    /// Output system readiness score and status
    fn output_system_readiness(&self, readiness: &SystemReadiness) {
        println!();
        println!("🎯 SYSTEM READINESS SCORE:");
        println!("───────────────────────");

        let status_icon = match readiness.status {
            ReadinessStatus::Ready => "🟢",
            ReadinessStatus::PartiallyReady => "🟡",
            ReadinessStatus::NotReady => "🔴",
        };

        println!("{} Score: {}/100", status_icon, readiness.score);
        println!("Status: {:?}", readiness.status);
        println!("Assessment: {}", readiness.description);
        println!();
    }

    /// Output actionable recommendations with detailed steps
    fn output_actionable_recommendations(&self, recommendations: &[ActionableRecommendation]) {
        if recommendations.is_empty() {
            return;
        }

        println!("📋 ACTIONABLE RECOMMENDATIONS:");
        println!("────────────────────────────");
        println!();

        for (i, rec) in recommendations.iter().enumerate() {
            let priority_icon = match rec.priority {
                RecommendationPriority::Critical => "🔴 CRITICAL",
                RecommendationPriority::High => "🟠 HIGH",
                RecommendationPriority::Medium => "🟡 MEDIUM",
                RecommendationPriority::Low => "🟢 LOW",
            };

            println!("{}. {} [{}]", i + 1, rec.title, priority_icon);
            println!("   Category: {}", rec.category);
            println!("   Description: {}", rec.description);

            if !rec.steps.is_empty() {
                println!("   Steps:");
                for (j, step) in rec.steps.iter().enumerate() {
                    println!("     {}. {}", j + 1, step);
                }
            }

            if !rec.links.is_empty() {
                println!("   Resources:");
                for link in &rec.links {
                    println!("     • {}", link);
                }
            }

            if i < recommendations.len() - 1 {
                println!();
            }
        }

        println!();
        println!(
            "💡 TIP: Address critical and high priority recommendations first for maximum impact."
        );
        println!();
    }
}