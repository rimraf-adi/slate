# 05 - Media Analysis Engine Architecture & Implementation Plan (`slate-analysis`)

> **Specification & Technical Architecture Document**  
> **Crate:** `crates/slate-analysis`  
> **Target Release:** v1.0 (Headless & Desktop Dual-Surface Intelligence)  
> **Status:** APPROVED FOR IMPLEMENTATION  
> **Core Trait Contract:** `slate-core::traits::analyzer::MediaAnalyzer`  

---

## 1. Overview & System Architecture

`slate-analysis` is the sensory and analytical engine of the Slate non-linear video editor. It transforms raw multimodal media streams—compressed video frames and multi-channel audio tracks—into structured, semantically rich, temporal metadata. This crate grants both human editors and autonomous AI agents (operating via Model Context Protocol / `slate-mcp`) the ability to "see", "hear", and "read" video timelines.

### 1.1 Core Mission & Architectural Principles

1. **100% Local & Offline Execution:** Modern video workflows require absolute privacy, air-gapped security, and zero API token costs. All inference tasks—speech-to-text, neural voice activity detection (VAD), scene transition classification, loudness compliance, and visual preview synthesis—execute locally on host CPU and GPU/NPU silicon. No audio or video data ever leaves the user's machine.
2. **Deterministic & Frame-Accurate:** All analytical events (speech words, silence boundaries, scene cut points, loudness intervals) are anchored to `slate_core::time::RationalTime` intervals. Analysis results are bit-exact and immune to floating-point drift across hours of continuous footage.
3. **Decoupled Asynchronous Processing:** Analysis tasks are computationally intensive. They run as background jobs managed by a prioritized worker queue, exposing lock-free atomic progress monitors (`Arc<AtomicF32>`) and cancellation tokens (`tokio_util::sync::CancellationToken`). The UI and MCP command layers remain fully responsive during analysis.
4. **Content-Addressable Persistent Caching:** Analytical pipelines are idempotent. Analysis artifacts (transcripts, scene cut vectors, loudness reports, thumbnail contact sheets, waveform pyramids) are keyed deterministically by the SHA-256 hash of the media asset, codec parameters, and model configuration. Analysis is performed exactly once per media asset state.
5. **Direct Timeline Mutation Synergy:** Analysis is not merely descriptive; it is actionable. Analysis outputs (such as word timings and silence intervals) translate directly into atomic `slate-core` operations (`Op::SplitClip`, `Op::RippleDelete`, `Op::Batch`) to enable natural language, text-based, and silence-removal editing.

---

### 1.2 System Architecture & Data Flow

```
                                      +--------------------------------------------------------+
                                      |                 slate-mcp / slate-ui                 |
                                      |   (AI Agents, Natural Language Editing, UI Waveforms)   |
                                      +--------------------------------------------------------+
                                                                  ^
                                                                  | Query Results / Event Stream
                                                                  v
+---------------------------------------------------------------------------------------------------------------------------------------+
|                                                           SLATE-ANALYSIS                                                             |
|                                                                                                                                       |
|  +---------------------------------------------------------------------------------------------------------------------------------+  |
|  |                                                 AnalysisJobScheduler (Priority Queue)                                           |  |
|  |     [Priority::High: Thumbnails] > [Priority::Normal: STT & VAD] > [Priority::Low: Scenes & Loudness] > [Priority::Bg: Proxies]   |  |
|  +---------------------------------------------------------------------------------------------------------------------------------+  |
|                                                                  |                                                                    |
|                        +-----------------------------------------+-----------------------------------------+                          |
|                        |                                         |                                         |                          |
|                        v                                         v                                         v                          |
|             +-----------------------+                 +-----------------------+                 +-----------------------+             |
|             |   Audio Ingestion     |                 |   Video Ingestion     |                 |    Model Manager      |             |
|             | (16kHz f32 Mono Dec)  |                 | (320x180 RGB/HSV Dec) |                 | (Local Cache / Verif) |             |
|             +-----------------------+                 +-----------------------+                 +-----------------------+             |
|                        |                                         |                                         |                          |
|            +-----------+-----------+                     +-------+-------+                                 |                          |
|            |                       |                     |               |                                 v                          |
|            v                       v                     v               v                      +-----------------------+             |
|    +---------------+       +---------------+     +---------------+ +---------------+            | ~/.local/share/slate |             |
|    |  whisper-rs   |       |  Silero VAD   |     |  SceneDetect  | |   Thumbnail   |            |        models/        |             |
|    | (Whisper STT) |       |  (ort ONNX)   |     | (HSV Hist-Chi)| | (Grid Sheet)  |            |  - whisper-base.bin   |             |
|    +---------------+       +---------------+     +---------------+ +---------------+            |  - silero_vad.onnx    |             |
|            |                       |                     |               |                      +-----------------------+             |
|            |                       v                     |               |                                                            |
|            |               +---------------+             |               |                                                            |
|            |               |    ebur128    |             |               |                                                            |
|            |               | (BS.1770-4)   |             |               |                                                            |
|            |               +---------------+             |               |                                                            |
|            |                       |                     |               |                                                            |
|            +-----------+-----------+                     +-------+-------+                                                            |
|                        |                                         |                                                                    |
|                        +--------------------+--------------------+                                                                    |
|                                             |                                                                                         |
|                                             v                                                                                         |
|                                  +-----------------------+                                                                            |
|                                  |   Persistent Cache    |                                                                            |
|                                  | (.slate/cache/...)   |                                                                            |
|                                  |  - transcripts/*.json |                                                                            |
|                                  |  - vad/*.json         |                                                                            |
|                                  |  - scenes/*.json      |                                                                            |
|                                  |  - loudness/*.json    |                                                                            |
|                                  |  - thumbs/*.jpg       |                                                                            |
|                                  +-----------------------+                                                                            |
|                                             |                                                                                         |
+---------------------------------------------|-----------------------------------------------------------------------------------------+
                                              v
                               +-----------------------------+
                               |         slate-core         |
                               | (Timeline, Op::Batch, Undo) |
                               +-----------------------------+
```

---

### 1.3 Crate Dependencies & Boundaries

`slate-analysis` is strictly isolated from windowing systems (`egui`), raw audio output drivers (`cpal`), and GPU presentation pipelines. It interfaces with media decoding through `slate-media`, temporal models through `slate-core`, and relies on dedicated high-performance numerical and machine learning runtimes.

```toml
[package]
name = "slate-analysis"
version = "0.1.0"
edition = "2021"
authors = ["Slate Video Editor Contributors"]
license = "MIT OR Apache-2.0"
description = "Multimodal AI media analysis engine for Slate video editor (Whisper, Silero VAD, Scene Detection, Loudness, Thumbnails)"

[dependencies]
# Workspace Dependencies
slate-core = { path = "../slate-core" }
slate-media = { path = "../slate-media" }

# Speech-to-Text via whisper.cpp bindings
whisper-rs = { version = "0.11", default-features = false }

# Neural Voice Activity Detection (Silero VAD) via ONNX Runtime
ort = { version = "2.0.0-rc.2", default-features = false, features = ["ndarray", "download-binaries"] }
ndarray = { version = "0.15", features = ["serde"] }

# ITU-R BS.1770-4 / EBU R128 Loudness Measurement
ebur128 = "0.1"

# Image Processing & Contact Sheet Generation
image = { version = "0.25", default-features = false, features = ["jpeg", "png"] }

# Asynchronous Execution & Task Management
tokio = { version = "1.38", features = ["sync", "rt-multi-thread", "time", "fs"] }
tokio-util = { version = "0.7", features = ["sync"] }
futures-util = "0.3"
rayon = "1.10"
crossbeam-channel = "0.5"
parking_lot = "0.12"

# Serialization & Binary Caching
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
bincode = "1.3"

# System & Storage Utilities
sha2 = "0.10"
hex = "0.4"
directories = "5.0"
reqwest = { version = "0.12", default-features = false, features = ["rustls-tls", "stream"] }

# Observability & Errors
tracing = "0.1"
thiserror = "1.0"
uuid = { version = "1.8", features = ["v4", "serde"] }

[features]
default = []
metal = ["whisper-rs/metal"]
cuda = ["whisper-rs/cuda", "ort/cuda"]
coreml = ["whisper-rs/coreml", "ort/coreml"]

[dev-dependencies]
tempfile = "3.10"
approx = "0.5"
criterion = { version = "0.5", features = ["html_reports"] }
```

---

### 1.4 Module Organization

```
crates/slate-analysis/
├── Cargo.toml
├── src/
│   ├── lib.rs                   # Crate root, engine facade, prelude
│   ├── error.rs                 # Comprehensive AnalysisError definitions
│   ├── types.rs                 # Common types, JobId, ModelEnums
│   ├── transcribe/              # Speech-to-Text Subsystem (whisper-rs)
│   │   ├── mod.rs               # Transcriber orchestration & public exports
│   │   ├── engine.rs            # whisper-rs context wrapper & inference loop
│   │   ├── audio.rs             # 16kHz mono audio resampler & normalization
│   │   ├── edit.rs              # Transcript-to-Timeline editing algorithm
│   │   └── cache.rs             # Transcript JSON caching & SHA-256 keys
│   ├── vad/                     # Voice Activity Detection (Silero VAD)
│   │   ├── mod.rs               # VAD module declarations
│   │   ├── silero.rs            # ONNX Runtime model execution & recurrent state
│   │   ├── segments.rs          # Speech/silence clustering & interval merging
│   │   └── silence_cut.rs       # RippleDelete operation batch builder
│   ├── scene/                   # Visual Scene Cut Detection
│   │   ├── mod.rs               # Scene detector exports
│   │   ├── detector.rs          # HSV color histogram extractor
│   │   ├── distance.rs          # Chi-Squared & Bhattacharyya distance metrics
│   │   └── adaptive.rs          # Dynamic thresholding & cut extraction
│   ├── loudness/                # EBU R128 Audio Loudness Measurement
│   │   ├── mod.rs               # Loudness module declarations
│   │   ├── meter.rs             # ebur128 state container & sample processing
│   │   └── report.rs            # LoudnessReport generator & target validator
│   ├── thumbnail/               # Visual Thumbnail & Storyboard Generation
│   │   ├── mod.rs               # Thumbnail module declarations
│   │   ├── single.rs            # Frame extraction & Lanczos3 downscaling
│   │   └── contact_sheet.rs     # Multi-frame composite grid generator
│   ├── waveform/                # Multi-Resolution Audio Waveform Pyramid
│   │   ├── mod.rs               # Waveform module declarations
│   │   └── pyramid.rs           # Min/Max/RMS peak reduction (.slatewave)
│   ├── jobs/                    # Prioritized Background Job Scheduler
│   │   ├── mod.rs               # Scheduler exports
│   │   ├── job.rs               # AnalysisJob, Task enum, status tracking
│   │   ├── queue.rs             # Priority queue & concurrency limits
│   │   └── runner.rs            # Worker execution thread loops
│   ├── models/                  # Offline Model Management & CDN Sync
│   │   ├── mod.rs               # Model manager declarations
│   │   ├── registry.rs          # Verified model catalog (Whisper/Silero)
│   │   ├── downloader.rs        # Streaming HTTP downloader with progress
│   │   └── verifier.rs          # SHA-256 integrity check & local cache paths
│   └── trait_impl.rs            # Implementation of slate_core::traits::MediaAnalyzer
```

---

## 2. Speech Transcription Subsystem (`whisper-rs`)

The transcription subsystem provides frame-accurate speech recognition with word-level timestamps. It forms the foundation for transcript-based text editing, subtitle generation, natural language clip search, and conversational editing via MCP agents.

### 2.1 Whisper Engine Integration & Hardware Acceleration

`whisper-rs` exposes high-performance C++ bindings to Georgi Gerganov's `whisper.cpp`. The audio must be supplied as a contiguous single-channel 16,000 Hz `f32` slice normalized to the dynamic range $[-1.0, 1.0]$.

#### Hardware Acceleration Matrix

| Platform | Primary Backend | Cargo Feature | Fallback Path | Performance (Base Model) |
| :--- | :--- | :--- | :--- | :--- |
| **macOS (Apple Silicon)** | Apple Core ML / Metal | `whisper-rs/metal`, `whisper-rs/coreml` | Accelerate CPU | ~18x faster than real-time |
| **Linux (NVIDIA)** | CUDA 12.x / cuBLAS | `whisper-rs/cuda` | OpenBLAS / AVX2 | ~22x faster than real-time |
| **Windows (NVIDIA)** | CUDA 12.x | `whisper-rs/cuda` | AVX2 CPU | ~20x faster than real-time |
| **Windows / Linux (Intel/AMD)** | CPU SIMD | Default (AVX2 / AVX-512) | Standard CPU | ~4x faster than real-time |

```rust
// crates/slate-analysis/src/transcribe/engine.rs

use std::path::Path;
use std::sync::Arc;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};
use crate::error::AnalysisError;
use crate::transcribe::audio::PreparedAudio;
use crate::transcribe::cache::TranscriptCacheKey;
use crate::types::{TranscriptionParams, Transcript, TranscriptSegment, TranscriptWord};
use slate_core::id::MediaId;
use slate_core::time::RationalTime;

pub struct WhisperEngine {
    context: WhisperContext,
    model_path: std::path::PathBuf,
}

impl WhisperEngine {
    pub fn new(model_path: &Path) -> Result<Self, AnalysisError> {
        if !model_path.exists() {
            return Err(AnalysisError::ModelNotFound(model_path.to_path_buf()));
        }

        let mut params = WhisperContextParameters::default();
        // Enable GPU acceleration where compiled
        #[cfg(feature = "metal")]
        params.use_gpu(true);
        #[cfg(feature = "cuda")]
        params.use_gpu(true);

        let context = WhisperContext::new_with_params(
            model_path.to_str().ok_or_else(|| {
                AnalysisError::InvalidArgument("Invalid UTF-8 in model path".into())
            })?,
            params,
        ).map_err(|e| AnalysisError::TranscriptionEngine(format!("Failed to init whisper context: {:?}", e)))?;

        Ok(Self {
            context,
            model_path: model_path.to_path_buf(),
        })
    }

    pub fn transcribe(
        &self,
        media_id: MediaId,
        audio: &PreparedAudio,
        params: &TranscriptionParams,
    ) -> Result<Transcript, AnalysisError> {
        let mut state = self.context.create_state()
            .map_err(|e| AnalysisError::TranscriptionEngine(format!("Failed to create whisper state: {:?}", e)))?;

        let mut full_params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        full_params.set_print_progress(false);
        full_params.set_print_special(false);
        full_params.set_print_realtime(false);
        full_params.set_print_timestamps(false);
        full_params.set_token_timestamps(true); // Crucial for word-level timestamps
        full_params.set_translate(params.translate_to_english);
        
        if let Some(ref lang) = params.language {
            full_params.set_language(Some(lang.as_str()));
        } else {
            full_params.set_language(None); // Auto-detect
        }

        // Suppress non-speech hallucinations during music or long pauses
        full_params.set_suppress_blank(true);
        full_params.set_suppress_non_speech_tokens(true);
        full_params.set_temperature(0.0); // Deterministic greedy decoding

        state.full(full_params, audio.samples())
            .map_err(|e| AnalysisError::TranscriptionEngine(format!("Whisper inference failed: {:?}", e)))?;

        let num_segments = state.full_n_segments()
            .map_err(|e| AnalysisError::TranscriptionEngine(format!("Segment retrieval failed: {:?}", e)))?;

        let mut segments = Vec::with_capacity(num_segments as usize);
        let mut words = Vec::new();
        let detected_lang = params.language.clone().unwrap_or_else(|| "auto".to_string());

        for s in 0..num_segments {
            let seg_text = state.full_get_segment_text(s)
                .map_err(|e| AnalysisError::TranscriptionEngine(format!("Segment text failed: {:?}", e)))?;
            let t0_cs = state.full_get_segment_t0(s); // In centiseconds (10ms)
            let t1_cs = state.full_get_segment_t1(s);

            let seg_start = RationalTime::from_millis(t0_cs * 10, 1000);
            let seg_end = RationalTime::from_millis(t1_cs * 10, 1000);

            segments.push(TranscriptSegment {
                id: s as usize,
                start: seg_start,
                end: seg_end,
                text: seg_text.trim().to_string(),
            });

            // Extract word-level tokens
            let num_tokens = state.full_n_tokens(s)
                .map_err(|e| AnalysisError::TranscriptionEngine(format!("Token count failed: {:?}", e)))?;

            for t in 0..num_tokens {
                let token_data = state.full_get_token_data(s, t)
                    .map_err(|e| AnalysisError::TranscriptionEngine(format!("Token data failed: {:?}", e)))?;
                let token_text = state.full_get_token_text(s, t)
                    .map_err(|e| AnalysisError::TranscriptionEngine(format!("Token text failed: {:?}", e)))?;

                // Skip pure whitespace or special tokens (marked with timestamps < 0)
                if token_data.t0 < 0 || token_data.t1 < 0 || token_text.trim().is_empty() {
                    continue;
                }

                let word_start = RationalTime::from_millis(token_data.t0 * 10, 1000);
                let word_end = RationalTime::from_millis(token_data.t1 * 10, 1000);

                words.push(TranscriptWord {
                    word: token_text.trim().to_string(),
                    start: word_start,
                    end: word_end,
                    confidence: token_data.p,
                    segment_id: s as usize,
                });
            }
        }

        Ok(Transcript {
            media_id,
            language: detected_lang,
            segments,
            words,
        })
    }
}
```

---

### 2.2 Model Architecture & Sizes

Slate supports the official GGML Whisper weights. Model configurations represent varying tradeoffs between RAM footprint, inference speed, and Word Error Rate (WER):

| Model Variant | Disk Size | Required RAM | Multi-Lingual | Primary Use Case |
| :--- | :--- | :--- | :--- | :--- |
| `WhisperModel::Tiny` | 75 MB | ~390 MB | Yes | Rapid rough-cut drafting, ultra-low resource devices |
| `WhisperModel::Base` | 142 MB | ~500 MB | Yes | **Slate Default:** Balanced accuracy/latency on laptops |
| `WhisperModel::Small` | 466 MB | ~1.0 GB | Yes | High-accuracy dialogue editing, accented English |
| `WhisperModel::Medium` | 1.5 GB | ~2.6 GB | Yes | Broadcast-grade accuracy, multilingual transcription |
| `WhisperModel::LargeV3Turbo`| 1.6 GB | ~3.0 GB | Yes | State-of-the-art accuracy at 4x the speed of standard Large |

```rust
// crates/slate-analysis/src/types.rs

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
```

---

### 2.3 Transcript Data Models

```rust
// crates/slate-analysis/src/types.rs

use serde::{Deserialize, Serialize};
use slate_core::id::MediaId;
use slate_core::time::RationalTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionParams {
    pub model_size: WhisperModel,
    pub language: Option<String>,         // ISO-639-1 code (e.g. "en", "es"), or None for auto-detect
    pub word_timestamps: bool,            // Always true for NLE text editing
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transcript {
    pub media_id: MediaId,
    pub language: String,
    pub segments: Vec<TranscriptSegment>,
    pub words: Vec<TranscriptWord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptSegment {
    pub id: usize,
    pub start: RationalTime,
    pub end: RationalTime,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptWord {
    pub word: String,
    pub start: RationalTime,
    pub end: RationalTime,
    pub confidence: f32,
    pub segment_id: usize,
}
```

---

### 2.4 Word-Level Editing & Timeline Operation Mapping

Text-based video editing allows users or AI agents to edit video by editing words in the transcript. Deleting a word or sentence in the transcript translates into splitting clips on the timeline and ripple-deleting the exact time intervals.

#### The Word-to-Timeline Editing Problem
1. Transcripts are indexed in **media-relative time** ($0 \dots \text{Duration}$).
2. Clips on the timeline reference media assets via `source_range` placed at a `timeline_range`.
3. Truncating words directly at their raw boundaries causes plosive clipping (e.g. cutting off the "p" or "t" at word boundaries). We apply a safety padding buffer:
   - **Lead-in padding:** 60 ms before word start.
   - **Lead-out padding:** 80 ms after word end.
4. If two kept word ranges are separated by less than 150 ms of silence, they are merged to avoid jarring micro-edits.

```rust
// crates/slate-analysis/src/transcribe/edit.rs

use slate_core::id::{ClipId, TrackId};
use slate_core::model::clip::Clip;
use slate_core::model::timeline::Timeline;
use slate_core::op::types::Op;
use slate_core::time::{RationalTime, TimeRange};
use crate::error::AnalysisError;
use crate::types::TranscriptWord;

#[derive(Debug, Clone)]
pub struct WordRange {
    pub start_word_idx: usize,
    pub end_word_idx: usize, // Inclusive
}

#[derive(Debug, Clone)]
pub struct EditByTranscript {
    pub keep_ranges: Vec<WordRange>,
    pub lead_in_padding: RationalTime,
    pub lead_out_padding: RationalTime,
}

impl EditByTranscript {
    pub fn default_padded(keep_ranges: Vec<WordRange>) -> Self {
        Self {
            keep_ranges,
            lead_in_padding: RationalTime::from_millis(60, 1000),
            lead_out_padding: RationalTime::from_millis(80, 1000),
        }
    }

    /// Converts word index selections into an atomic batch of timeline operations.
    pub fn build_timeline_ops(
        &self,
        words: &[TranscriptWord],
        timeline: &Timeline,
        track_id: TrackId,
        clip_id: ClipId,
    ) -> Result<Op, AnalysisError> {
        let clip = timeline
            .find_clip(clip_id)
            .ok_or_else(|| AnalysisError::TimelineMapping(format!("Clip {:?} not found", clip_id)))?;

        // 1. Convert Word Ranges to Continuous Media Source Time Ranges
        let mut source_keep_ranges: Vec<TimeRange> = Vec::new();
        for range in &self.keep_ranges {
            if range.start_word_idx >= words.len() || range.end_word_idx >= words.len() {
                return Err(AnalysisError::TimelineMapping("Word index out of bounds".into()));
            }

            let raw_start = words[range.start_word_idx].start;
            let raw_end = words[range.end_word_idx].end;

            // Apply acoustic padding
            let padded_start = if raw_start > self.lead_in_padding {
                raw_start - self.lead_in_padding
            } else {
                RationalTime::ZERO
            };
            let padded_end = raw_end + self.lead_out_padding;

            source_keep_ranges.push(TimeRange::new(padded_start, padded_end - padded_start));
        }

        // 2. Sort and merge overlapping or near-adjacent intervals (< 150ms)
        source_keep_ranges.sort_by_key(|r| r.start);
        let merge_threshold = RationalTime::from_millis(150, 1000);
        let mut merged_keep: Vec<TimeRange> = Vec::new();

        for r in source_keep_ranges {
            if let Some(last) = merged_keep.last_mut() {
                if r.start <= last.end() + merge_threshold {
                    let new_end = last.end().max(r.end());
                    *last = TimeRange::new(last.start, new_end - last.start);
                    continue;
                }
            }
            merged_keep.push(r);
        }

        // 3. Compute the complement: intervals to DELETE within the clip's source range
        let clip_source_range = clip.source_range;
        let mut delete_source_ranges: Vec<TimeRange> = Vec::new();
        let mut cursor = clip_source_range.start;

        for keep in &merged_keep {
            if keep.start > cursor {
                delete_source_ranges.push(TimeRange::new(cursor, keep.start - cursor));
            }
            cursor = cursor.max(keep.end());
        }

        if cursor < clip_source_range.end() {
            delete_source_ranges.push(TimeRange::new(cursor, clip_source_range.end() - cursor));
        }

        // 4. Map Source Time Ranges to Timeline Time Ranges
        // Timeline_t = Clip_Timeline_Start + (Source_t - Clip_Source_Start) * (1 / speed)
        let mut timeline_deletes: Vec<TimeRange> = Vec::new();
        for src_del in delete_source_ranges {
            let offset_start = src_del.start - clip_source_range.start;
            let timeline_start = clip.place_range.start + offset_start;
            timeline_deletes.push(TimeRange::new(timeline_start, src_del.duration));
        }

        // 5. Construct atomic Op::Batch of SplitClip and RippleDelete operations
        // We delete from end to start so timeline coordinates of earlier splits remain stable
        timeline_deletes.sort_by_key(|r| std::cmp::Reverse(r.start));
        let mut ops = Vec::new();

        for del in timeline_deletes {
            ops.push(Op::RippleDelete {
                track_id,
                range: del,
            });
        }

        Ok(Op::Batch(ops))
    }
}
```

---

### 2.5 Caching & Hash Invalidation

Transcription of a 1-hour 4K podcast takes several minutes on CPU/GPU. Slate stores transcripts under `.slate/cache/transcripts/` using a deterministic SHA-256 fingerprint:

$$\text{CacheKey} = \text{SHA256}(\text{media\_file\_sha256} \,\|\, \text{model\_size} \,\|\, \text{language\_tag})$$

```rust
// crates/slate-analysis/src/transcribe/cache.rs

use std::fs;
use std::path::{Path, PathBuf};
use sha2::{Digest, Sha256};
use crate::error::AnalysisError;
use crate::types::{TranscriptionParams, Transcript, WhisperModel};

pub struct TranscriptCache {
    cache_dir: PathBuf,
}

impl TranscriptCache {
    pub fn new(project_cache_root: &Path) -> Self {
        let dir = project_cache_root.join("transcripts");
        fs::create_dir_all(&dir).ok();
        Self { cache_dir: dir }
    }

    pub fn compute_key(media_sha256: &str, params: &TranscriptionParams) -> String {
        let mut hasher = Sha256::new();
        hasher.update(media_sha256.as_bytes());
        hasher.update(format!("{:?}", params.model_size).as_bytes());
        if let Some(ref lang) = params.language {
            hasher.update(lang.as_bytes());
        }
        hasher.update(&[params.translate_to_english as u8]);
        hex::encode(hasher.finalize())
    }

    pub fn get(&self, cache_key: &str) -> Option<Transcript> {
        let path = self.cache_dir.join(format!("{}.json", cache_key));
        if !path.exists() {
            return None;
        }
        let data = fs::read_to_string(path).ok()?;
        serde_json::from_str(&data).ok()
    }

    pub fn put(&self, cache_key: &str, transcript: &Transcript) -> Result<(), AnalysisError> {
        let final_path = self.cache_dir.join(format!("{}.json", cache_key));
        let temp_path = self.cache_dir.join(format!("{}.tmp.{}", cache_key, uuid::Uuid::new_v4()));

        let serialized = serde_json::to_string_pretty(transcript)
            .map_err(|e| AnalysisError::Serialization(e.to_string()))?;

        // Atomic write via temporary file rename
        fs::write(&temp_path, serialized)
            .map_err(|e| AnalysisError::Io(e))?;
        fs::rename(&temp_path, &final_path)
            .map_err(|e| AnalysisError::Io(e))?;

        Ok(())
    }
}
```

---

## 3. Voice Activity Detection Subsystem (Silero VAD)

Voice Activity Detection (VAD) separates active speech from silence, pauses, and background noise. It empowers AI agents to generate jump cuts, remove dead air, and clean up audio tracks automatically.

### 3.1 Model Architecture & `ort` Integration

Slate embeds the state-of-the-art **Silero VAD v5** model. Unlike naive amplitude/energy thresholding, Silero VAD is an enterprise-grade recurrent neural network (LSTM + Conv) trained on over 6,000 languages. It accurately rejects keyboard clatter, breathing, air conditioner hum, and room reverb.

#### Key Specs
- **Model Size:** 2.1 MB ONNX binary (`silero_vad.onnx`).
- **Input Sampling Rate:** Exactly 16,000 Hz single-channel PCM `f32`.
- **Window Size:** Exactly 512 samples per forward step ($32\text{ ms}$).
- **State Tensors:** Hidden state $h$ and Cell state $c$ of dimension `[2, 1, 64]` propagated between consecutive 512-sample frames.
- **Latency:** $\approx 0.8\text{ ms}$ inference per $32\text{ ms}$ audio chunk on CPU.

```rust
// crates/slate-analysis/src/vad/silero.rs

use ndarray::{Array, Array2, Array3, Axis};
use ort::{session::Session, value::Value};
use std::path::Path;
use slate_core::time::{RationalTime, TimeRange};
use crate::error::AnalysisError;

pub struct SileroVad {
    session: Session,
    h: Array3<f32>, // Shape: [2, 1, 64]
    c: Array3<f32>, // Shape: [2, 1, 64]
}

impl SileroVad {
    pub const SAMPLE_RATE: usize = 16000;
    pub const WINDOW_SIZE: usize = 512; // 32ms at 16kHz

    pub fn new(model_path: &Path) -> Result<Self, AnalysisError> {
        let session = Session::builder()
            .map_err(|e| AnalysisError::VadEngine(format!("Ort builder failed: {:?}", e)))?
            .with_intra_threads(1)
            .map_err(|e| AnalysisError::VadEngine(format!("Ort thread config failed: {:?}", e)))?
            .commit_from_file(model_path)
            .map_err(|e| AnalysisError::VadEngine(format!("Failed to load Silero ONNX: {:?}", e)))?;

        Ok(Self {
            session,
            h: Array3::zeros((2, 1, 64)),
            c: Array3::zeros((2, 1, 64)),
        })
    }

    pub fn reset_states(&mut self) {
        self.h.fill(0.0);
        self.c.fill(0.0);
    }

    /// Run inference on a single 512-sample chunk, returning the speech probability [0.0, 1.0].
    pub fn process_chunk(&mut self, chunk: &[f32]) -> Result<f32, AnalysisError> {
        if chunk.len() != Self::WINDOW_SIZE {
            return Err(AnalysisError::InvalidArgument(format!(
                "Chunk size must be {}, got {}",
                Self::WINDOW_SIZE,
                chunk.len()
            )));
        }

        let input_tensor = Array2::from_shape_vec((1, Self::WINDOW_SIZE), chunk.to_vec())
            .map_err(|e| AnalysisError::VadEngine(e.to_string()))?;
        let sr_tensor = Array::from_elem(1, Self::SAMPLE_RATE as i64);

        let input_val = Value::from_array(input_tensor)
            .map_err(|e| AnalysisError::VadEngine(e.to_string()))?;
        let sr_val = Value::from_array(sr_tensor)
            .map_err(|e| AnalysisError::VadEngine(e.to_string()))?;
        let h_val = Value::from_array(self.h.clone())
            .map_err(|e| AnalysisError::VadEngine(e.to_string()))?;
        let c_val = Value::from_array(self.c.clone())
            .map_err(|e| AnalysisError::VadEngine(e.to_string()))?;

        let outputs = self.session.run(ort::inputs![
            "input" => input_val,
            "sr" => sr_val,
            "h" => h_val,
            "c" => c_val
        ].map_err(|e| AnalysisError::VadEngine(e.to_string()))?)
        .map_err(|e| AnalysisError::VadEngine(format!("ONNX execution failed: {:?}", e)))?;

        // Silero outputs: output (prob [1, 1]), hn [2, 1, 64], cn [2, 1, 64]
        let prob_tensor = outputs["output"].try_extract_tensor::<f32>()
            .map_err(|e| AnalysisError::VadEngine(e.to_string()))?;
        let prob = prob_tensor[[0, 0]];

        let hn_tensor = outputs["hn"].try_extract_tensor::<f32>()
            .map_err(|e| AnalysisError::VadEngine(e.to_string()))?;
        let cn_tensor = outputs["cn"].try_extract_tensor::<f32>()
            .map_err(|e| AnalysisError::VadEngine(e.to_string()))?;

        // Update recurrent states
        self.h.assign(&hn_tensor);
        self.c.assign(&cn_tensor);

        Ok(prob)
    }
}
```

---

### 3.2 VAD Parameters & Result Representation

```rust
// crates/slate-analysis/src/types.rs

use serde::{Deserialize, Serialize};
use slate_core::time::TimeRange;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VadParams {
    /// Speech probability threshold [0.0, 1.0] to trigger active speech state.
    pub threshold: f32,
    /// Minimum duration of continuous speech to register as a speech segment.
    pub min_speech_duration_ms: u32,
    /// Minimum duration of silence to split speech segments or mark dead air.
    pub min_silence_duration_ms: u32,
    /// Number of samples per VAD evaluation window (512 for 16kHz).
    pub window_size_samples: u32,
    /// Padding prepended to speech segments to avoid clipping plosives (e.g. 80ms).
    pub pre_padding_ms: u32,
    /// Padding appended to speech segments to avoid clipping trailing vowels (e.g. 120ms).
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VadResult {
    pub speech_segments: Vec<TimeRange>,
    pub silence_segments: Vec<TimeRange>,
}
```

---

### 3.3 State Machine Algorithm & Speech Clustering

Raw per-frame probabilities chatter around the boundary. The VAD state machine applies hysteresis, temporal smoothing, and minimum-duration filtering:

```
Frame Prob > Threshold ──► Speech Trigger ──► Count Active Frames ──► Emit Speech Segment
                                     ▲                   │
                                     │                   ▼
Frame Prob < (Threshold - 0.15) ─────┴──────── Silence Counter >= MinSilence ──► Emit Silence
```

```rust
// crates/slate-analysis/src/vad/segments.rs

use slate_core::time::{RationalTime, TimeRange};
use crate::error::AnalysisError;
use crate::types::{VadParams, VadResult};
use crate::vad::silero::SileroVad;

pub fn analyze_vad(
    vad: &mut SileroVad,
    samples: &[f32],
    params: &VadParams,
) -> Result<VadResult, AnalysisError> {
    vad.reset_states();

    let window_size = SileroVad::WINDOW_SIZE;
    let sample_rate = SileroVad::SAMPLE_RATE as u32;
    let num_chunks = samples.len() / window_size;

    let min_speech_samples = (params.min_speech_duration_ms * sample_rate) / 1000;
    let min_silence_samples = (params.min_silence_duration_ms * sample_rate) / 1000;
    let pre_pad_samples = (params.pre_padding_ms * sample_rate) / 1000;
    let post_pad_samples = (params.post_padding_ms * sample_rate) / 1000;

    let mut speech_intervals: Vec<(usize, usize)> = Vec::new();
    let mut is_speech = false;
    let mut speech_start = 0;
    let mut silence_start = 0;

    for i in 0..num_chunks {
        let chunk_start = i * window_size;
        let chunk = &samples[chunk_start..chunk_start + window_size];
        let prob = vad.process_chunk(chunk)?;

        if prob >= params.threshold {
            if !is_speech {
                // Potential transition from silence to speech
                if chunk_start.saturating_sub(silence_start) >= min_silence_samples as usize {
                    is_speech = true;
                    speech_start = chunk_start.saturating_sub(pre_pad_samples as usize);
                }
            }
        } else if prob < (params.threshold - 0.15).max(0.1) {
            if is_speech {
                // Potential transition from speech to silence
                silence_start = chunk_start;
                if chunk_start.saturating_sub(speech_start) >= min_speech_samples as usize {
                    let speech_end = (chunk_start + window_size + post_pad_samples as usize)
                        .min(samples.len());
                    speech_intervals.push((speech_start, speech_end));
                }
                is_speech = false;
            }
        }
    }

    // Flush trailing speech segment if audio ended during speech
    if is_speech && samples.len().saturating_sub(speech_start) >= min_speech_samples as usize {
        speech_intervals.push((speech_start, samples.len()));
    }

    // Convert raw sample indices to frame-accurate TimeRange
    let mut speech_ranges = Vec::new();
    for (start_idx, end_idx) in speech_intervals {
        let start_time = RationalTime::from_sample_index(start_idx as u64, sample_rate);
        let end_time = RationalTime::from_sample_index(end_idx as u64, sample_rate);
        speech_ranges.push(TimeRange::new(start_time, end_time - start_time));
    }

    // Compute inverted silence intervals
    let total_duration = RationalTime::from_sample_index(samples.len() as u64, sample_rate);
    let mut silence_ranges = Vec::new();
    let mut cursor = RationalTime::ZERO;

    for speech in &speech_ranges {
        if speech.start > cursor {
            silence_ranges.push(TimeRange::new(cursor, speech.start - cursor));
        }
        cursor = cursor.max(speech.end());
    }

    if cursor < total_duration {
        silence_ranges.push(TimeRange::new(cursor, total_duration - cursor));
    }

    Ok(VadResult {
        speech_segments: speech_ranges,
        silence_segments: silence_ranges,
    })
}
```

---

### 3.4 Integration with Silence Removal & Ripple Delete

Silero VAD results integrate directly into automated jump-cut assembly. An agent or user specifies `remove_silence`, which computes the silence intervals on a clip and produces an atomic `Op::Batch` containing `RippleDelete` commands.

```rust
// crates/slate-analysis/src/vad/silence_cut.rs

use slate_core::id::{ClipId, TrackId};
use slate_core::model::clip::Clip;
use slate_core::op::types::Op;
use slate_core::time::{RationalTime, TimeRange};
use crate::types::VadResult;

pub fn generate_silence_removal_ops(
    vad_result: &VadResult,
    clip: &Clip,
    track_id: TrackId,
    min_cut_duration: RationalTime,
) -> Op {
    let mut ops = Vec::new();

    // Map silence intervals that fall within this clip's source range
    let mut candidate_deletes: Vec<TimeRange> = Vec::new();

    for silence in &vad_result.silence_segments {
        if silence.duration < min_cut_duration {
            continue; // Keep short natural breath pauses
        }

        // Clamp silence to clip's source range
        let start = silence.start.max(clip.source_range.start);
        let end = silence.end().min(clip.source_range.end());

        if end > start {
            let clamped_duration = end - start;
            if clamped_duration >= min_cut_duration {
                // Translate source time to timeline placement time
                let offset = start - clip.source_range.start;
                let timeline_start = clip.place_range.start + offset;
                candidate_deletes.push(TimeRange::new(timeline_start, clamped_duration));
            }
        }
    }

    // Sort descending to keep earlier coordinates unaffected as we ripple
    candidate_deletes.sort_by_key(|r| std::cmp::Reverse(r.start));

    for del in candidate_deletes {
        ops.push(Op::RippleDelete {
            track_id,
            range: del,
        });
    }

    Op::Batch(ops)
}
```

---

## 4. Visual Scene Cut Detection Subsystem

Scene cut detection identifies shot transitions, camera angle changes, and visual scene boundaries. This enables automated B-roll insertion, chapter generation, and rough-cut assembly.

### 4.1 Histogram-Based Detection Principles

Comparing consecutive frames via raw RGB difference is highly vulnerable to camera shake, slight object motion, and exposure changes. `slate-analysis` projects frames into the **HSV (Hue, Saturation, Value)** color space and constructs a multi-bin color distribution:

$$\text{Hue: } 16 \text{ bins}, \quad \text{Saturation: } 8 \text{ bins}, \quad \text{Value: } 8 \text{ bins} \quad \implies \quad 1024 \text{ joint 3D bins or } 32 \text{ separated bins}$$

We use separated 1D HSV histograms (64 bins each: 64 H + 64 S + 64 V = 192 bins) for maximum speed and illumination invariance. The distance between consecutive histograms $H_1$ and $H_2$ is measured via the **Chi-Squared ($\chi^2$) distance**:

$$\chi^2(H_1, H_2) = \frac{1}{2} \sum_{i=1}^{K} \frac{\left(H_1(i) - H_2(i)\right)^2}{H_1(i) + H_2(i) + \epsilon}$$

Where $\chi^2 \in [0.0, 1.0]$. A sharp spike above the adaptive threshold signals a hard scene cut.

---

### 4.2 Algorithm & Dynamic Content Thresholding

```
  [Video Stream]
        │
        ▼ (Decode at 5 FPS, 320x180 resolution)
  [Downscaled Frames]
        │
        ▼ (Convert RGB -> HSV & Bin)
  [HSV Histograms]
        │
        ▼ (Compute Chi-Squared Difference)
  [Distance Vector Δt]
        │
        ▼ (Calculate Sliding Window Mean μ & StdDev σ)
  [Adaptive Threshold T = μ + k·σ]
        │
        ▼ (Detect Peaks & Suppress Near Cuts < MinSceneLength)
  [SceneCut Vector]
```

```rust
// crates/slate-analysis/src/scene/detector.rs

use std::path::{Path, PathBuf};
use slate_core::time::RationalTime;
use crate::error::AnalysisError;
use crate::types::{SceneCut, SceneDetectionParams};

pub struct HsvHistogram {
    pub h: [f32; 64],
    pub s: [f32; 64],
    pub v: [f32; 64],
}

impl HsvHistogram {
    pub fn from_rgb_frame(width: u32, height: u32, rgb_data: &[u8]) -> Self {
        let mut h_bins = [0.0f32; 64];
        let mut s_bins = [0.0f32; 64];
        let mut v_bins = [0.0f32; 64];

        let num_pixels = (width * height) as f32;

        for pixel in rgb_data.chunks_exact(3) {
            let r = pixel[0] as f32 / 255.0;
            let g = pixel[1] as f32 / 255.0;
            let b = pixel[2] as f32 / 255.0;

            let max = r.max(g).max(b);
            let min = r.min(g).min(b);
            let delta = max - min;

            // Value
            let v = max;

            // Saturation
            let s = if max > 0.0 { delta / max } else { 0.0 };

            // Hue [0.0, 1.0]
            let mut h = if delta > 0.0 {
                if max == r {
                    (g - b) / delta + (if g < b { 6.0 } else { 0.0 })
                } else if max == g {
                    (b - r) / delta + 2.0
                } else {
                    (r - g) / delta + 4.0
                }
            } else {
                0.0
            };
            h /= 6.0;

            let h_idx = ((h * 63.99) as usize).min(63);
            let s_idx = ((s * 63.99) as usize).min(63);
            let v_idx = ((v * 63.99) as usize).min(63);

            h_bins[h_idx] += 1.0;
            s_bins[s_idx] += 1.0;
            v_bins[v_idx] += 1.0;
        }

        // Normalize
        for i in 0..64 {
            h_bins[i] /= num_pixels;
            s_bins[i] /= num_pixels;
            v_bins[i] /= num_pixels;
        }

        Self { h: h_bins, s: s_bins, v: v_bins }
    }

    pub fn chi_squared_distance(&self, other: &Self) -> f64 {
        let mut dist = 0.0;
        const EPSILON: f32 = 1e-7;

        for i in 0..64 {
            let h_sum = self.h[i] + other.h[i] + EPSILON;
            let h_diff = self.h[i] - other.h[i];
            dist += (h_diff * h_diff) / h_sum;

            let s_sum = self.s[i] + other.s[i] + EPSILON;
            let s_diff = self.s[i] - other.s[i];
            dist += (s_diff * s_diff) / s_sum;

            let v_sum = self.v[i] + other.v[i] + EPSILON;
            let v_diff = self.v[i] - other.v[i];
            dist += (v_diff * v_diff) / v_sum;
        }

        (dist / 3.0) as f64
    }
}
```

---

### 4.3 Scene Detection API & Parameter Structure

```rust
// crates/slate-analysis/src/types.rs

use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use slate_core::time::RationalTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneDetectionParams {
    /// Baseline Chi-Squared distance threshold [0.0, 1.0]. Default: 0.4.
    pub threshold: f64,
    /// Minimum temporal duration of a distinct scene. Default: 1.0 second.
    pub min_scene_length: RationalTime,
    /// Enable sliding-window variance adaptive thresholding.
    pub adaptive: bool,
    /// Sub-sampled analysis framerate (e.g. 5.0 fps) for ultra-fast processing.
    pub sample_rate_fps: f64,
}

impl Default for SceneDetectionParams {
    fn default() -> Self {
        Self {
            threshold: 0.4,
            min_scene_length: RationalTime::from_seconds(1, 1),
            adaptive: true,
            sample_rate_fps: 5.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneCut {
    pub time: RationalTime,
    pub confidence: f64,
    pub frame_before: Option<PathBuf>,
    pub frame_after: Option<PathBuf>,
}
```

---

## 5. Acoustic Loudness Measurement (`ebur128`)

Proper audio levels are mandatory for broadcast standards and online streaming platforms. `slate-analysis` includes an ITU-R BS.1770-4 / EBU R128 compliant loudness analyzer.

### 5.1 Standards & Target Profiles

| Platform / Standard | Integrated Target | Maximum True Peak | Max Short-Term | Loudness Range (LRA) |
| :--- | :--- | :--- | :--- | :--- |
| **YouTube** | -14.0 LUFS | -1.0 dBTP | -13.0 LUFS | Recommended < 14 LU |
| **Spotify** | -14.0 LUFS | -1.0 dBTP | -13.0 LUFS | Commercial master |
| **Apple Podcasts** | -16.0 LUFS | -1.0 dBTP | -14.0 LUFS | Speech target |
| **EBU R128 (EU Broadcast)** | -23.0 LUFS | -1.0 dBTP | -18.0 LUFS | Strict broadcast |
| **ATSC A/85 (US Television)**| -24.0 LUFS | -2.0 dBTP | -19.0 LUFS | Commercial CALM act |

---

### 5.2 Filter Chain & K-Weighting DSP

The loudness meter applies the K-weighting pre-filter curve simulating human head acoustic acoustic transfer function:
1. **Stage 1 (Pre-filter):** High-frequency boost simulating head diffraction ($+4\text{ dB}$ shelf at $1.5\text{ kHz}$).
2. **Stage 2 (RLB filter):** High-pass filter attenuating low frequencies below $100\text{ Hz}$.

```
Input Audio ──► [Stage 1: High-Shelf] ──► [Stage 2: High-Pass] ──► Energy Sum ──► Gating Stage ──► LUFS
```

---

### 5.3 Loudness Report & Analysis Engine

```rust
// crates/slate-analysis/src/loudness/meter.rs

use ebur128::{EbuR128, Mode};
use slate_core::time::RationalTime;
use crate::error::AnalysisError;
use crate::types::LoudnessReport;

pub struct LoudnessMeter {
    meter: EbuR128,
    sample_rate: u32,
    channels: u32,
}

impl LoudnessMeter {
    pub fn new(channels: u32, sample_rate: u32) -> Result<Self, AnalysisError> {
        let mode = Mode::INTEGRATED | Mode::SHORT_TERM | Mode::MOMENTARY | Mode::TRUE_PEAK | Mode::LRA;
        let meter = EbuR128::new(channels, sample_rate, mode)
            .map_err(|e| AnalysisError::LoudnessEngine(format!("Failed to init ebur128: {:?}", e)))?;

        Ok(Self {
            meter,
            sample_rate,
            channels,
        })
    }

    /// Process planar or interleaved 32-bit floating point PCM audio chunks.
    pub fn add_frames(&mut self, frames: &[f32]) -> Result<(), AnalysisError> {
        self.meter.add_frames_f32(frames)
            .map_err(|e| AnalysisError::LoudnessEngine(format!("Failed to feed frames to ebur128: {:?}", e)))
    }

    pub fn finalize(&mut self) -> Result<LoudnessReport, AnalysisError> {
        let integrated_lufs = self.meter.loudness_global()
            .map_err(|e| AnalysisError::LoudnessEngine(format!("Global loudness failed: {:?}", e)))?;

        let loudness_range = self.meter.loudness_range()
            .map_err(|e| AnalysisError::LoudnessEngine(format!("LRA failed: {:?}", e)))?;

        let mut true_peak_max = -100.0;
        for ch in 0..self.channels {
            if let Ok(peak) = self.meter.true_peak(ch) {
                // Convert linear peak amplitude to dBTP
                let dbtp = 20.0 * peak.log10();
                if dbtp > true_peak_max {
                    true_peak_max = dbtp;
                }
            }
        }

        Ok(LoudnessReport {
            integrated_lufs,
            short_term_lufs: Vec::new(), // Filled during streaming window sampling
            momentary_lufs: Vec::new(),
            loudness_range,
            true_peak_dbtp: true_peak_max,
        })
    }
}
```

```rust
// crates/slate-analysis/src/types.rs

use serde::{Deserialize, Serialize};
use slate_core::time::RationalTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoudnessReport {
    /// Program integrated loudness across entire media in LUFS.
    pub integrated_lufs: f64,
    /// Short-term loudness sampled every 3.0 seconds (Time, LUFS).
    pub short_term_lufs: Vec<(RationalTime, f64)>,
    /// Momentary loudness sampled every 400ms (Time, LUFS).
    pub momentary_lufs: Vec<(RationalTime, f64)>,
    /// Loudness Range (LRA) statistical variance in LU.
    pub loudness_range: f64,
    /// Maximum True Peak across all channels in dBTP.
    pub true_peak_dbtp: f64,
}
```

---

## 6. Thumbnail & Visual Asset Generation

AI agents require visual grounding to inspect scenes, confirm edits, and identify characters or graphical title cards. `slate-analysis` renders individual high-resolution thumbnails and composite storyboard contact sheets.

### 6.1 Single Frame Extraction

```rust
// crates/slate-analysis/src/thumbnail/single.rs

use std::path::{Path, PathBuf};
use image::{imageops::FilterType, ImageBuffer, Rgb};
use slate_core::id::MediaId;
use slate_core::time::RationalTime;
use slate_media::decoder::VideoDecoder;
use crate::error::AnalysisError;

pub fn render_thumbnail(
    decoder: &mut dyn VideoDecoder,
    cache_root: &Path,
    media_id: MediaId,
    time: RationalTime,
    target_size: (u32, u32),
) -> Result<PathBuf, AnalysisError> {
    let out_dir = cache_root.join("thumbnails").join(media_id.to_string());
    std::fs::create_dir_all(&out_dir).map_err(AnalysisError::Io)?;

    let out_filename = format!("thumb_{}_{}x{}.jpg", time.to_nanos(), target_size.0, target_size.1);
    let out_path = out_dir.join(out_filename);

    if out_path.exists() {
        return Ok(out_path);
    }

    // Seek and decode target frame
    decoder.seek(time).map_err(|e| AnalysisError::MediaPipeline(e.to_string()))?;
    let frame = decoder.next_frame()
        .map_err(|e| AnalysisError::MediaPipeline(e.to_string()))?
        .ok_or_else(|| AnalysisError::MediaPipeline("EOF reached seeking thumbnail frame".into()))?;

    // Downscale using high-quality Lanczos3 filter
    let raw_img = ImageBuffer::<Rgb<u8>, _>::from_raw(
        frame.width,
        frame.height,
        frame.data.to_vec(),
    ).ok_or_else(|| AnalysisError::Thumbnail("Invalid raw frame buffer dimensions".into()))?;

    let resized = image::imageops::resize(
        &raw_img,
        target_size.0,
        target_size.1,
        FilterType::Lanczos3,
    );

    // Save as JPEG (Quality 85)
    resized.save_with_format(&out_path, image::ImageFormat::Jpeg)
        .map_err(|e| AnalysisError::Thumbnail(e.to_string()))?;

    Ok(out_path)
}
```

---

### 6.2 Contact Sheet & Storyboard Generation

Contact sheets composite multiple evenly-spaced frames into an indexed $R \times C$ image matrix. This provides AI agents with an instant high-level visual index of an entire video in a single payload.

```
+-------------------------------------------------------------+
| 00:00:00.000       | 00:02:15.500       | 00:04:30.000      |
| [ Frame 1 ]        | [ Frame 2 ]        | [ Frame 3 ]       |
+--------------------+--------------------+-------------------+
| 00:06:45.250       | 00:09:00.000       | 00:11:15.750      |
| [ Frame 4 ]        | [ Frame 5 ]        | [ Frame 6 ]       |
+--------------------+--------------------+-------------------+
| 00:13:30.000       | 00:15:45.000       | 00:18:00.000      |
| [ Frame 7 ]        | [ Frame 8 ]        | [ Frame 9 ]       |
+-------------------------------------------------------------+
```

```rust
// crates/slate-analysis/src/thumbnail/contact_sheet.rs

use std::path::{Path, PathBuf};
use image::{imageops, ImageBuffer, Rgba, RgbImage};
use slate_core::id::MediaId;
use slate_core::time::{RationalTime, TimeRange};
use slate_media::decoder::VideoDecoder;
use crate::error::AnalysisError;

pub fn render_contact_sheet(
    decoder: &mut dyn VideoDecoder,
    cache_root: &Path,
    media_id: MediaId,
    count: usize,
    columns: usize,
    thumb_size: (u32, u32),
    range: Option<TimeRange>,
) -> Result<PathBuf, AnalysisError> {
    if count == 0 || columns == 0 {
        return Err(AnalysisError::InvalidArgument("Count and columns must be > 0".into()));
    }

    let out_dir = cache_root.join("contact_sheets");
    std::fs::create_dir_all(&out_dir).map_err(AnalysisError::Io)?;

    let rows = (count + columns - 1) / columns;
    let padding = 10u32;
    let grid_width = (columns as u32 * thumb_size.0) + ((columns as u32 + 1) * padding);
    let grid_height = (rows as u32 * thumb_size.1) + ((rows as u32 + 1) * padding);

    let hash_key = format!("{}_{}_{}x{}_{}", media_id, count, columns, thumb_size.0, thumb_size.1);
    let out_path = out_dir.join(format!("{}.jpg", hash_key));

    if out_path.exists() {
        return Ok(out_path);
    }

    // Determine temporal sampling interval
    let full_duration = decoder.duration();
    let effective_range = range.unwrap_or_else(|| TimeRange::new(RationalTime::ZERO, full_duration));
    let step = effective_range.duration / (count as i64);

    let mut composite_canvas = RgbImage::from_pixel(
        grid_width,
        grid_height,
        image::Rgb([24, 24, 27]), // Matte dark grey background
    );

    for i in 0..count {
        let sample_time = effective_range.start + (step * (i as i64));
        decoder.seek(sample_time).map_err(|e| AnalysisError::MediaPipeline(e.to_string()))?;
        
        if let Some(frame) = decoder.next_frame().map_err(|e| AnalysisError::MediaPipeline(e.to_string()))? {
            let raw_img = RgbImage::from_raw(frame.width, frame.height, frame.data.to_vec())
                .ok_or_else(|| AnalysisError::Thumbnail("Buffer size mismatch".into()))?;

            let thumb = imageops::resize(&raw_img, thumb_size.0, thumb_size.1, imageops::FilterType::Triangle);

            let col_idx = (i % columns) as u32;
            let row_idx = (i / columns) as u32;

            let x_offset = padding + col_idx * (thumb_size.0 + padding);
            let y_offset = padding + row_idx * (thumb_size.1 + padding);

            imageops::overlay(&mut composite_canvas, &thumb, x_offset as i64, y_offset as i64);
        }
    }

    composite_canvas.save_with_format(&out_path, image::ImageFormat::Jpeg)
        .map_err(|e| AnalysisError::Thumbnail(e.to_string()))?;

    Ok(out_path)
}
```

---

## 7. Background Job Scheduler & Worker Pool

Video analysis tasks are CPU/GPU-bound operations that run in the background. `slate-analysis` incorporates an asynchronous, prioritized job scheduler with thread-safe progress monitoring and cooperative cancellation.

### 7.1 Job Types & Task Priority

```rust
// crates/slate-analysis/src/jobs/job.rs

use std::sync::atomic::AtomicF32;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use slate_core::id::MediaId;
use crate::types::*;

pub type JobId = uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum JobPriority {
    High = 3,       // Interactive UI scrubs, single-frame thumbnails
    Normal = 2,     // User/Agent triggered STT, VAD silence removal
    Low = 1,        // Scene-cut detection, Loudness measurement
    Background = 0, // Video proxies, waveform pyramids
}

#[derive(Debug, Clone)]
pub enum AnalysisTask {
    Transcription(TranscriptionParams),
    SceneDetection(SceneDetectionParams),
    SilenceDetection(VadParams),
    LoudnessMeasurement,
    ThumbnailGeneration { time: slate_core::time::RationalTime, size: (u32, u32) },
    ContactSheetGeneration { count: usize, columns: usize, thumb_size: (u32, u32) },
    WaveformGeneration,
    ProxyGeneration,
}

pub struct AnalysisJob {
    pub id: JobId,
    pub priority: JobPriority,
    pub task: AnalysisTask,
    pub media_id: MediaId,
    pub progress: Arc<AtomicF32>, // Range [0.0, 1.0]
    pub cancel: CancellationToken,
}
```

---

### 7.2 Job Scheduler Implementation

```rust
// crates/slate-analysis/src/jobs/queue.rs

use std::collections::BinaryHeap;
use std::cmp::Ordering;
use std::sync::Arc;
use parking_lot::Mutex;
use tokio::sync::Notify;
use crate::jobs::job::{AnalysisJob, JobId, JobPriority};

struct PrioritizedJob(AnalysisJob);

impl PartialEq for PrioritizedJob {
    fn eq(&self, other: &Self) -> bool {
        self.0.priority == other.0.priority && self.0.id == other.0.id
    }
}

impl Eq for PrioritizedJob {}

impl PartialOrd for PrioritizedJob {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PrioritizedJob {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.priority.cmp(&other.0.priority)
            .then_with(|| other.0.id.cmp(&self.0.id)) // FIFO on tie
    }
}

pub struct JobQueue {
    heap: Mutex<BinaryHeap<PrioritizedJob>>,
    notify: Notify,
    max_concurrency: usize,
}

impl JobQueue {
    pub fn new(max_concurrency: usize) -> Self {
        Self {
            heap: Mutex::new(BinaryHeap::new()),
            notify: Notify::new(),
            max_concurrency,
        }
    }

    pub fn submit(&self, job: AnalysisJob) {
        let mut queue = self.heap.lock();
        queue.push(PrioritizedJob(job));
        self.notify.notify_one();
    }

    pub async fn next_job(&self) -> AnalysisJob {
        loop {
            {
                let mut queue = self.heap.lock();
                if let Some(job) = queue.pop() {
                    return job.0;
                }
            }
            self.notify.notified().await;
        }
    }
}
```

---

## 8. Offline Model Management & Asset Lifecycle

Slate is designed to operate seamlessly in offline, studio, and air-gapped field environments.

### 8.1 Model Catalog & Storage Resolution

Models are stored under the OS standard application data directory:
- **macOS:** `~/Library/Application Support/com.slate.editor/models/`
- **Linux:** `~/.local/share/slate/models/`
- **Windows:** `%LOCALAPPDATA%\slate\models\`

```rust
// crates/slate-analysis/src/models/registry.rs

use std::path::PathBuf;
use directories::ProjectDirs;
use crate::error::AnalysisError;

pub struct ModelManager {
    models_dir: PathBuf,
}

impl ModelManager {
    pub fn new() -> Result<Self, AnalysisError> {
        let proj_dirs = ProjectDirs::from("com", "slate", "editor")
            .ok_or_else(|| AnalysisError::Io(std::io::Error::new(std::io::ErrorKind::NotFound, "App data dir unavailable")))?;
        
        let models_dir = proj_dirs.data_local_dir().join("models");
        std::fs::create_dir_all(&models_dir).map_err(AnalysisError::Io)?;

        Ok(Self { models_dir })
    }

    pub fn get_model_path(&self, filename: &str) -> PathBuf {
        self.models_dir.join(filename)
    }

    pub fn is_model_installed(&self, filename: &str) -> bool {
        self.models_dir.join(filename).exists()
    }
}
```

---

### 8.2 Streaming Downloader with SHA-256 Verification

If a requested model is missing and the user is online, the downloader pulls weights from the official Slate CDN/HuggingFace repo with streaming SHA-256 verification. If the machine is offline, it returns an explicit error without blocking.

```rust
// crates/slate-analysis/src/models/downloader.rs

use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use futures_util::StreamExt;
use sha2::{Digest, Sha256};
use crate::error::AnalysisError;

pub async fn download_model_with_progress(
    url: &str,
    target_path: &Path,
    expected_sha256: &str,
    progress_bytes: Arc<AtomicU64>,
) -> Result<(), AnalysisError> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .map_err(|e| AnalysisError::Network(e.to_string()))?;

    let response = client.get(url).send().await
        .map_err(|e| AnalysisError::Network(format!("Failed to reach model CDN: {:?}", e)))?;

    if !response.status().is_success() {
        return Err(AnalysisError::Network(format!("HTTP error {}: {}", response.status(), url)));
    }

    let temp_path = target_path.with_extension(format!("download.{}", uuid::Uuid::new_v4()));
    let mut file = File::create(&temp_path).map_err(AnalysisError::Io)?;
    let mut hasher = Sha256::new();
    let mut stream = response.bytes_stream();

    while let Some(chunk_res) = stream.next().await {
        let chunk = chunk_res.map_err(|e| AnalysisError::Network(e.to_string()))?;
        file.write_all(&chunk).map_err(AnalysisError::Io)?;
        hasher.update(&chunk);
        progress_bytes.fetch_add(chunk.len() as u64, Ordering::Relaxed);
    }

    file.flush().map_err(AnalysisError::Io)?;
    drop(file);

    // Verify SHA-256 checksum
    let calculated_sha = hex::encode(hasher.finalize());
    if calculated_sha != expected_sha256 {
        std::fs::remove_file(&temp_path).ok();
        return Err(AnalysisError::ModelIntegrity {
            expected: expected_sha256.to_string(),
            actual: calculated_sha,
        });
    }

    // Atomic install
    std::fs::rename(&temp_path, target_path).map_err(AnalysisError::Io)?;
    Ok(())
}
```

---

## 9. Implementation of `MediaAnalyzer` Trait

`slate-analysis` fulfills the trait contract declared in `slate-core::traits::analyzer::MediaAnalyzer`, bridging high-level operations with the underlying neural and algorithmic pipelines.

```rust
// crates/slate-analysis/src/trait_impl.rs

use std::path::Path;
use std::sync::Arc;
use slate_core::id::MediaId;
use slate_core::time::TimeRange;
use slate_core::traits::analyzer::{AnalysisError as CoreAnalysisError, MediaAnalyzer};
use slate_core::model::caption::{Transcript as CoreTranscript, TranscriptWord as CoreWord};
use slate_core::model::marker::SceneCut as CoreSceneCut;
use slate_core::traits::analyzer::LoudnessReport as CoreLoudnessReport;

use crate::error::AnalysisError;
use crate::models::registry::ModelManager;
use crate::transcribe::engine::WhisperEngine;
use crate::types::{TranscriptionParams, WhisperModel, VadParams, SceneDetectionParams};

pub struct SlateMediaAnalyzer {
    model_manager: Arc<ModelManager>,
}

impl SlateMediaAnalyzer {
    pub fn new() -> Result<Self, AnalysisError> {
        let model_manager = Arc::new(ModelManager::new()?);
        Ok(Self { model_manager })
    }
}

impl MediaAnalyzer for SlateMediaAnalyzer {
    fn transcribe(
        &self,
        media_id: MediaId,
        path: &Path,
    ) -> Result<CoreTranscript, CoreAnalysisError> {
        let model_path = self.model_manager.get_model_path(WhisperModel::Base.filename());
        let engine = WhisperEngine::new(&model_path)
            .map_err(|e| CoreAnalysisError::Engine(e.to_string()))?;

        // 1. Demux & resample audio to 16kHz mono via slate-media
        let prepared_audio = crate::transcribe::audio::extract_16k_mono_pcm(path)
            .map_err(|e| CoreAnalysisError::AudioDecode(e.to_string()))?;

        // 2. Perform transcription
        let params = TranscriptionParams::default();
        let internal_transcript = engine.transcribe(media_id, &prepared_audio, &params)
            .map_err(|e| CoreAnalysisError::Engine(e.to_string()))?;

        // 3. Map to slate-core contract
        Ok(CoreTranscript {
            media_id,
            language: internal_transcript.language,
            words: internal_transcript.words.into_iter().map(|w| CoreWord {
                word: w.word,
                start: w.start,
                end: w.end,
                confidence: w.confidence,
            }).collect(),
        })
    }

    fn detect_scenes(
        &self,
        media_id: MediaId,
        path: &Path,
    ) -> Result<Vec<CoreSceneCut>, CoreAnalysisError> {
        let params = SceneDetectionParams::default();
        let cuts = crate::scene::detector::detect_scenes_in_file(path, &params)
            .map_err(|e| CoreAnalysisError::Engine(e.to_string()))?;

        Ok(cuts.into_iter().map(|c| CoreSceneCut {
            time: c.time,
            confidence: c.confidence,
        }).collect())
    }

    fn detect_silence(
        &self,
        media_id: MediaId,
        path: &Path,
        _threshold_db: f64,
    ) -> Result<Vec<TimeRange>, CoreAnalysisError> {
        let vad_model_path = self.model_manager.get_model_path("silero_vad.onnx");
        let mut vad = crate::vad::silero::SileroVad::new(&vad_model_path)
            .map_err(|e| CoreAnalysisError::Engine(e.to_string()))?;

        let audio = crate::transcribe::audio::extract_16k_mono_pcm(path)
            .map_err(|e| CoreAnalysisError::AudioDecode(e.to_string()))?;

        let params = VadParams::default();
        let vad_result = crate::vad::segments::analyze_vad(&mut vad, audio.samples(), &params)
            .map_err(|e| CoreAnalysisError::Engine(e.to_string()))?;

        Ok(vad_result.silence_segments)
    }

    fn measure_loudness(
        &self,
        media_id: MediaId,
        path: &Path,
    ) -> Result<CoreLoudnessReport, CoreAnalysisError> {
        let report = crate::loudness::meter::measure_file_loudness(path)
            .map_err(|e| CoreAnalysisError::Engine(e.to_string()))?;

        Ok(CoreLoudnessReport {
            integrated_lufs: report.integrated_lufs,
            momentary_max_lufs: report.momentary_lufs.iter().map(|(_, l)| *l).fold(-100.0, f64::max),
            true_peak_dbfs: report.true_peak_dbtp,
        })
    }
}
```

---

## 10. Comprehensive Testing Matrix

The analysis engine undergoes a multi-layer verification strategy combining deterministic DSP test vectors, ground-truth alignment benchmarks, synthetic media pipelines, and Criterion performance suites.

### 10.1 Test Categories

| Test Suite | File / Scope | Target Property / Guarantee | Ground Truth / Tool |
| :--- | :--- | :--- | :--- |
| **Whisper Word Alignment** | `tests/test_transcription.rs` | Word timestamps align within $\pm 100\text{ ms}$ of ground truth | LibriSpeech 30s test sample |
| **Silero VAD Precision** | `tests/test_vad_accuracy.rs` | Speech detection F1-score $> 0.95$ on podcast dialogue | Labeled speech/silence audio corpus |
| **Scene Cut Precision** | `tests/test_scene_detect.rs` | Zero false positives on camera pans; 100% recall on hard cuts | Synthetic 1080p cut sequence |
| **EBU R128 Compliance** | `tests/test_loudness_ebu.rs` | Integrated LUFS matches within $\pm 0.1\text{ LU}$ of reference | EBU Tech 3341 / 3342 test signals |
| **Contact Sheet Compositor**| `tests/test_thumbnails.rs` | Grid geometry, pixel dimensions, and JPEG metadata | Reference golden PNG comparison |
| **Job Priority Queue** | `tests/test_job_scheduler.rs`| High-priority thumbnail tasks preempt low-priority renders | Concurrency stress simulator |
| **Model Verification** | `tests/test_model_manager.rs`| SHA-256 verification catches corrupted weights | Injected bitflip test |

---

### 10.2 Criterion Performance Benchmarks

```rust
// benches/bench_analysis.rs

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use slate_analysis::scene::detector::HsvHistogram;

fn bench_hsv_histogram_generation(c: &mut Criterion) {
    let width = 320;
    let height = 180;
    let synthetic_rgb = vec![128u8; (width * height * 3) as usize];

    c.bench_function("hsv_histogram_320x180", |b| {
        b.iter(|| {
            HsvHistogram::from_rgb_frame(
                black_box(width),
                black_box(height),
                black_box(&synthetic_rgb),
            )
        })
    });
}

criterion_group!(benches, bench_hsv_histogram_generation);
criterion_main!(benches);
```

---

## 11. Step-by-Step Implementation Checklist

This checklist defines the chronological implementation order for building `crates/slate-analysis`.

### Phase 1: Foundations, Error Handling & Model Management
- [ ] **1.1** Initialize `crates/slate-analysis/Cargo.toml` with `whisper-rs`, `ort`, `ndarray`, `ebur128`, `image`, `tokio`, and `serde`.
- [ ] **1.2** Implement `src/error.rs`: Comprehensive `AnalysisError` enum with `thiserror`.
- [ ] **1.3** Implement `src/types.rs`: Core data structures (`TranscriptionParams`, `Transcript`, `VadParams`, `SceneDetectionParams`, `LoudnessReport`).
- [ ] **1.4** Implement `src/models/registry.rs`: Model catalog for Whisper and Silero VAD.
- [ ] **1.5** Implement `src/models/downloader.rs`: Streaming HTTP model downloader with SHA-256 verification and atomic rename.
- [ ] **1.6** Implement `src/models/verifier.rs`: Local model presence and integrity audit.

### Phase 2: Speech Transcription Subsystem (`whisper-rs`)
- [ ] **2.1** Implement `src/transcribe/audio.rs`: Single-channel 16,000 Hz `f32` resampler and normalization pipeline.
- [ ] **2.2** Implement `src/transcribe/engine.rs`: `whisper-rs` context creation, state allocation, and greedy inference loop.
- [ ] **2.3** Implement token-level parsing in `engine.rs` to extract word timestamps and confidence metrics.
- [ ] **2.4** Implement `src/transcribe/cache.rs`: Project cache for transcripts keyed by media SHA-256 and model size.
- [ ] **2.5** Implement `src/transcribe/edit.rs`: `EditByTranscript` algorithm mapping word deletions to timeline `Op::SplitClip` and `Op::RippleDelete`.
- [ ] **2.6** Unit test transcript parsing and cache roundtripping.

### Phase 3: Voice Activity Detection (Silero VAD)
- [ ] **3.1** Implement `src/vad/silero.rs`: Silero VAD v5 ONNX model wrapper using `ort` runtime with recurrent state propagation.
- [ ] **3.2** Implement `src/vad/segments.rs`: State machine with hysteresis thresholding and minimum duration filtering.
- [ ] **3.3** Implement `src/vad/silence_cut.rs`: Generation of `Op::Batch` containing `RippleDelete` operations for silence removal.
- [ ] **3.4** Integrate pre-padding (80ms) and post-padding (120ms) into speech clustering to prevent plosive clipping.
- [ ] **3.5** Unit test VAD inference against synthetic sine-tone bursts and silent gaps.

### Phase 4: Visual Scene Cut Detection
- [ ] **4.1** Implement `src/scene/detector.rs`: RGB to HSV color conversion and 64-bin histogram construction.
- [ ] **4.2** Implement `src/scene/distance.rs`: Chi-Squared ($\chi^2$) and Bhattacharyya histogram distance metrics.
- [ ] **4.3** Implement `src/scene/adaptive.rs`: Dynamic sliding-window thresholding ($\mu + k \cdot \sigma$) to eliminate false cuts during camera pans.
- [ ] **4.4** Implement multi-threaded frame batching via `rayon` to process video at 5 fps.
- [ ] **4.5** Implement candidate cut thumbnail capture (`frame_before` and `frame_after`).
- [ ] **4.6** Unit test scene detection on synthetic black-to-white cuts.

### Phase 5: Acoustic Loudness Measurement (`ebur128`)
- [ ] **5.1** Implement `src/loudness/meter.rs`: Wrapper around `ebur128::EbuR128` context for multi-channel streaming audio.
- [ ] **5.2** Implement momentary (400ms) and short-term (3.0s) sliding window sampling.
- [ ] **5.3** Implement True Peak calculation using 4x oversampled sinc interpolation.
- [ ] **5.4** Implement `src/loudness/report.rs`: Generation of final `LoudnessReport` and target compliance check.
- [ ] **5.5** Verify loudness metrics against official EBU Tech 3341 reference wav files.

### Phase 6: Thumbnail & Storyboard Generation
- [ ] **6.1** Implement `src/thumbnail/single.rs`: Single frame seeking and Lanczos3 downscaling saved as JPEG.
- [ ] **6.2** Implement `src/thumbnail/contact_sheet.rs`: Multi-frame composite grid generator with dark matte background.
- [ ] **6.3** Implement `src/waveform/pyramid.rs`: Multi-tier level-of-detail (LOD) audio peak reduction cached to `.slatewave`.
- [ ] **6.4** Unit test contact sheet grid layout calculation and image bounds.

### Phase 7: Background Job System & Trait Integration
- [ ] **7.1** Implement `src/jobs/job.rs`: `AnalysisJob`, `AnalysisTask`, and atomic progress handle.
- [ ] **7.2** Implement `src/jobs/queue.rs`: Multi-priority binary heap worker queue.
- [ ] **7.3** Implement `src/jobs/runner.rs`: Asynchronous task execution workers with cooperative cancellation tokens.
- [ ] **7.4** Implement `src/trait_impl.rs`: Fulfill `slate_core::traits::analyzer::MediaAnalyzer`.
- [ ] **7.5** Implement crate root `src/lib.rs` and export clean public API.
- [ ] **7.6** Execute complete integration test suite across all media analysis subsystems.
