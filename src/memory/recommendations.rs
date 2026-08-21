use crate::memory::pressure::MemoryPressure;
use crate::monitoring::ram::RamStats;

pub fn get_recommendations(stats: &RamStats, pressure: MemoryPressure) -> Vec<String> {
    let mut recs = Vec::new();

    match pressure {
        MemoryPressure::Low => {
            recs.push("Memory conditions are healthy. No optimization required.".to_string());
        }
        MemoryPressure::Moderate => {
            if stats.usage_pct > 75.0 {
                recs.push(format!("RAM usage is {:.0}%. Consider trimming idle background processes.", stats.usage_pct));
            }
            if stats.commit_limit_bytes > 0 && (stats.commit_used_bytes as f32 / stats.commit_limit_bytes as f32) > 0.80 {
                recs.push("Commit charge is elevated. Review high-memory apps in Processes tab.".to_string());
            }
        }
        MemoryPressure::High => {
            recs.push("High memory pressure detected. Recommended action: Smart Memory Optimization.".to_string());
            recs.push("Check for memory-heavy background tabs and startup services.".to_string());
        }
        MemoryPressure::Critical => {
            recs.push("CRITICAL: Available physical RAM is under 1 GB. Immediate optimization recommended.".to_string());
            recs.push("Close unused applications to prevent paging churn.".to_string());
        }
    }

    recs
}
