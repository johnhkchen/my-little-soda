use crate::metrics::MetricsTracker;
use anyhow::Result;
use serde_json;

pub struct MetricsCommand {
    pub hours: u64,
    pub detailed: bool,
    pub ci_mode: bool,
}

pub struct ExportMetricsCommand {
    pub hours: u64,
    pub output: Option<String>,
    pub ci_mode: bool,
}

impl MetricsCommand {
    pub fn new(hours: u64, detailed: bool) -> Self {
        Self {
            hours,
            detailed,
            ci_mode: false,
        }
    }

    pub fn with_ci_mode(mut self, ci_mode: bool) -> Self {
        self.ci_mode = ci_mode;
        self
    }

    pub async fn execute(&self) -> Result<()> {
        let tracker = MetricsTracker::new();

        if self.ci_mode {
            self.execute_ci_mode(&tracker).await
        } else {
            self.execute_interactive_mode(&tracker).await
        }
    }

    async fn execute_interactive_mode(&self, tracker: &MetricsTracker) -> Result<()> {
        println!("📊 MY LITTLE SODA METRICS - Integration Performance Analytics");
        println!("======================================================");
        println!();
        println!("⏰ Time window: {} hours", self.hours);
        println!("📈 Detailed: {}", self.detailed);
        println!();

        match tracker.calculate_metrics(Some(self.hours)).await {
            Ok(metrics) => {
                let report = tracker.format_metrics_report(&metrics, self.detailed);
                println!("{}", report);

                if self.detailed {
                    println!("\n🔍 Performance Analysis:");
                    match tracker.format_performance_report(Some(self.hours)).await {
                        Ok(perf_report) => println!("{}", perf_report),
                        Err(e) => println!("⚠️  Performance report unavailable: {}", e),
                    }
                }

                println!("\n✅ Metrics analysis complete");
            }
            Err(e) => {
                println!("❌ Error calculating metrics: {}", e);
            }
        }

        Ok(())
    }

    async fn execute_ci_mode(&self, tracker: &MetricsTracker) -> Result<()> {
        match tracker
            .export_metrics_for_monitoring(Some(self.hours))
            .await
        {
            Ok(export_data) => {
                let json_output = serde_json::to_string_pretty(&export_data)?;
                println!("{}", json_output);
            }
            Err(e) => {
                eprintln!("Error exporting metrics: {}", e);
                return Err(e.into());
            }
        }
        Ok(())
    }
}

impl ExportMetricsCommand {
    pub fn new(hours: u64, output: Option<String>) -> Self {
        Self {
            hours,
            output,
            ci_mode: false,
        }
    }

    pub fn with_ci_mode(mut self, ci_mode: bool) -> Self {
        self.ci_mode = ci_mode;
        self
    }

    pub async fn execute(&self) -> Result<()> {
        let tracker = MetricsTracker::new();

        if self.ci_mode {
            self.execute_ci_mode(&tracker).await
        } else {
            self.execute_interactive_mode(&tracker).await
        }
    }

    async fn execute_interactive_mode(&self, tracker: &MetricsTracker) -> Result<()> {
        println!("📊 MY LITTLE SODA EXPORT METRICS - JSON Format");
        println!("=========================================");
        println!();
        println!("⏰ Time window: {} hours", self.hours);
        if let Some(output) = &self.output {
            println!("📁 Output file: {output}");
        } else {
            println!("📁 Output: stdout");
        }
        println!();

        self.export_metrics(&tracker).await
    }

    async fn execute_ci_mode(&self, tracker: &MetricsTracker) -> Result<()> {
        self.export_metrics(&tracker).await
    }

    async fn export_metrics(&self, tracker: &MetricsTracker) -> Result<()> {
        match tracker
            .export_metrics_for_monitoring(Some(self.hours))
            .await
        {
            Ok(export_data) => {
                let json_output = serde_json::to_string_pretty(&export_data)?;

                if let Some(output_file) = &self.output {
                    std::fs::write(output_file, &json_output)?;
                    if !self.ci_mode {
                        println!("✅ Metrics exported to: {}", output_file);
                    }
                } else {
                    println!("{}", json_output);
                }
            }
            Err(e) => {
                if self.ci_mode {
                    eprintln!("Error exporting metrics: {}", e);
                    return Err(e.into());
                } else {
                    println!("❌ Error exporting metrics: {}", e);
                }
            }
        }
        Ok(())
    }
}
