use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Result of a diagnostic check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticResult {
    pub status: DiagnosticStatus,
    pub message: String,
    pub details: Option<String>,
    pub suggestion: Option<String>,
}

/// Status of a diagnostic check
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum DiagnosticStatus {
    Pass,
    Fail,
    Warning,
    Info,
}

/// Diagnostic report containing all check results
#[derive(Debug, Serialize, Deserialize)]
pub struct DiagnosticReport {
    pub summary: DiagnosticSummary,
    pub checks: HashMap<String, DiagnosticResult>,
    pub readiness: SystemReadiness,
    pub recommendations: Vec<ActionableRecommendation>,
}

/// Summary of diagnostic results
#[derive(Debug, Serialize, Deserialize)]
pub struct DiagnosticSummary {
    pub total_checks: usize,
    pub passed: usize,
    pub failed: usize,
    pub warnings: usize,
    pub info: usize,
}

/// System readiness assessment
#[derive(Debug, Serialize, Deserialize)]
pub struct SystemReadiness {
    pub score: u8,
    pub status: ReadinessStatus,
    pub description: String,
}

/// Overall system readiness status
#[derive(Debug, Serialize, Deserialize)]
pub enum ReadinessStatus {
    Ready,
    PartiallyReady,
    NotReady,
}

/// Actionable recommendation for fixing issues
#[derive(Debug, Serialize, Deserialize)]
pub struct ActionableRecommendation {
    pub priority: RecommendationPriority,
    pub category: String,
    pub title: String,
    pub description: String,
    pub steps: Vec<String>,
    pub links: Vec<String>,
}

/// Priority level for recommendations
#[derive(Debug, Serialize, Deserialize)]
pub enum RecommendationPriority {
    Critical,
    High,
    Medium,
    Low,
}