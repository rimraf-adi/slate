use crate::error::McpError;
use crate::types::MutationDiffSummary;
use slate_core::model::timeline::Timeline;

pub struct SafetyGuardrails;

impl SafetyGuardrails {
    /// 25% threshold constant
    pub const THRESHOLD_RATIO: f64 = 0.25;

    pub fn calculate_diff(
        timeline: &Timeline,
        deleted_duration_seconds: f64,
        clips_removed: usize,
    ) -> MutationDiffSummary {
        let total_seconds = timeline.duration().to_seconds().max(0.001);
        let total_clips = timeline.tracks.iter().map(|t| t.items.len()).sum::<usize>().max(1);

        let duration_ratio = (deleted_duration_seconds / total_seconds).clamp(0.0, 1.0);
        let clip_ratio = (clips_removed as f64 / total_clips as f64).clamp(0.0, 1.0);
        let mutation_ratio = duration_ratio.max(clip_ratio);

        MutationDiffSummary {
            clips_added: 0,
            clips_removed,
            clips_modified: 0,
            duration_change_seconds: -deleted_duration_seconds,
            total_duration_seconds: total_seconds,
            mutation_percentage: (mutation_ratio * 1000.0).round() / 10.0,
        }
    }

    /// Evaluates whether an intended deletion exceeds the 25% threshold.
    /// If it does and `confirm` is not true, returns `Err(McpError::SafetyGuardTriggered)`.
    pub fn verify_mutation(
        timeline: &Timeline,
        deleted_duration_seconds: f64,
        clips_removed: usize,
        confirm: bool,
    ) -> Result<MutationDiffSummary, McpError> {
        let diff = Self::calculate_diff(timeline, deleted_duration_seconds, clips_removed);
        let mutation_ratio = (diff.mutation_percentage / 100.0).clamp(0.0, 1.0);

        if mutation_ratio > Self::THRESHOLD_RATIO && !confirm {
            return Err(McpError::SafetyGuardTriggered {
                deleted_duration_seconds,
                total_timeline_seconds: diff.total_duration_seconds,
                percentage_deleted: diff.mutation_percentage,
                remedy: "Run with dry_run: true first to preview the edit diff, then provide confirm: true to apply."
                    .to_string(),
            });
        }

        Ok(diff)
    }
}
