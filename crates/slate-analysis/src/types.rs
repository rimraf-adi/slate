use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use slate_core::id::MediaId;
use slate_core::time::{RationalTime, TimeRange};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub enum WhisperModel {
    Tiny,
    Base,
    Small,
    Medium,
    LargeV3Turbo,
}

impl WhisperModel {
    pub fn filename(&self) -> &'static str {
        match self {
            WhisperModel::Tiny => "ggml-tiny.bin",
            WhisperModel::Base => "ggml-base.bin",
            WhisperModel::Small => "ggml-small.bin",
            WhisperModel::Medium => "ggml-medium.bin",
            WhisperModel::LargeV3Turbo => "ggml-large-v3-turbo.bin",
        }
    }

    pub fn expected_sha256(&self) -> &'static str {
        match self {
            WhisperModel::Tiny => "bd577a113a86444586340398fdd8d99e09d022690e8240d0641386ebd78b29d0",
            WhisperModel::Base => "46076c1f12408e4b88bd5e0f07002485e6f8b1800e171b2194be0ab305417c9f",
            WhisperModel::Small => "55356645c2b3661acac22e95a9db524497b334a447db95f0b56155e82b489d68",
            WhisperModel::Medium => "fd9727b6e171760d8473050c4585bc66033100f3531392d43fe61a614f2916f3",
            WhisperModel::LargeV3Turbo => "1fc70f7aaab9e91d310efcbcc1db83e1bfae34515286438ee2e4b33539e0eb3e",
        }
    }

    pub fn download_url(&self) -> &'static str {
        match self {
            WhisperModel::Tiny => "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.bin",
            WhisperModel::Base => "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin",
            WhisperModel::Small => "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin",
            WhisperModel::Medium => "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-medium.bin",
            WhisperModel::LargeV3Turbo => "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-turbo.bin",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TranscriptionParams {
    pub model_size: WhisperModel,
    pub language: Option<String>,
    pub word_timestamps: bool,
    pub translate_to_english: bool,
}

impl Default for TranscriptionParams {
    fn default() -> Self {
        Self {
            model_size: WhisperModel::Base,
            language: None,
            word_timestamps: true,
            translate_to_english: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct Transcript {
    pub media_id: MediaId,
    pub language: String,
    pub segments: Vec<TranscriptSegment>,
    pub words: Vec<TranscriptWord>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct TranscriptSegment {
    pub id: usize,
    pub start: RationalTime,
    pub end: RationalTime,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct TranscriptWord {
    pub word: String,
    pub start: RationalTime,
    pub end: RationalTime,
    pub confidence: f32,
    pub segment_id: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct WordRange {
    pub start_word_idx: usize,
    pub end_word_idx: usize, // Inclusive
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct VadParams {
    /// Speech probability / energy threshold [0.0, 1.0] to trigger active speech state.
    pub threshold: f32,
    /// Minimum duration of continuous speech to register as a speech segment.
    pub min_speech_duration_ms: u32,
    /// Minimum duration of silence to split speech segments or mark dead air.
    pub min_silence_duration_ms: u32,
    /// Number of samples per VAD evaluation window (512 for 16kHz).
    pub window_size_samples: u32,
    /// Padding prepended to speech segments to avoid clipping plosives.
    pub pre_padding_ms: u32,
    /// Padding appended to speech segments to avoid clipping trailing vowels.
    pub post_padding_ms: u32,
}

impl Default for VadParams {
    fn default() -> Self {
        Self {
            threshold: 0.5,
            min_speech_duration_ms: 250,
            min_silence_duration_ms: 300,
            window_size_samples: 512,
            pre_padding_ms: 80,
            post_padding_ms: 120,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct VadResult {
    pub speech_segments: Vec<TimeRange>,
    pub silence_segments: Vec<TimeRange>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SceneParams {
    /// Difference threshold between consecutive frames [0.0, 1.0] (default: 0.35)
    pub threshold: f32,
    /// Minimum frames between two detected scene cuts to avoid false positive flutter
    pub min_scene_length_frames: u64,
    /// Number of histogram bins per color channel (e.g. 16)
    pub color_bins: usize,
}

impl Default for SceneParams {
    fn default() -> Self {
        Self {
            threshold: 0.35,
            min_scene_length_frames: 15,
            color_bins: 16,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum SceneCutType {
    Cut,
    DissolveStart,
    DissolveEnd,
    FadeIn,
    FadeOut,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct SceneCut {
    pub frame: u64,
    pub time: RationalTime,
    pub cut_type: SceneCutType,
    pub score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct LoudnessReport {
    pub integrated_lufs: f64,
    pub loudness_range_lu: f64,
    pub true_peak_dbfs: f64,
    pub momentary_max_lufs: f64,
    pub short_term_max_lufs: f64,
    pub target_lufs: f64,
    pub gain_adjustment_db: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ContactSheetParams {
    pub cols: usize,
    pub rows: usize,
    pub thumb_width: u32,
    pub thumb_height: u32,
    pub time_range: Option<TimeRange>,
}

impl Default for ContactSheetParams {
    fn default() -> Self {
        Self {
            cols: 4,
            rows: 4,
            thumb_width: 320,
            thumb_height: 180,
            time_range: None,
        }
    }
}
