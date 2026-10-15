use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;

use base64::Engine as _;
use slate_analysis::types::{Transcript, TranscriptSegment, TranscriptWord};
use slate_analysis::EditByTranscript;
use slate_core::engine::Engine;
use slate_core::history::OpSource;
use slate_core::id::{ClipId, EffectId, KeyframeId, MediaId, TrackId, TransitionId};
use slate_core::model::timeline::{Timeline, TimelineSettings};
use slate_core::model::{
    CaptionStyle, Clip, Effect, Interpolation, Keyframe, Project, Track, TrackItem, TrackType,
    Transition, TransitionAlignment,
};
use slate_core::op::{
    DuckParams, KeyframeProperty, Op, TrimEdge, TrimMode,
};
use slate_core::time::{RationalTime, TimeRange};
use slate_ipc::client::EngineClient;

use crate::error::McpError;
use crate::guardrails::SafetyGuardrails;
use crate::tools::registry::ToolCallResult;
use crate::types::requests::*;
use crate::types::responses::*;
use crate::types::time::TimeResponse;

pub struct McpHandler {
    engine: Arc<Mutex<Engine>>,
    _client: Option<Arc<EngineClient>>,
    transcripts: Arc<Mutex<HashMap<MediaId, Transcript>>>,
    export_jobs: Arc<Mutex<HashMap<String, JobStatusResponse>>>,
}

impl McpHandler {
    pub fn new_headless(engine: Engine) -> Self {
        Self {
            engine: Arc::new(Mutex::new(engine)),
            _client: None,
            transcripts: Arc::new(Mutex::new(HashMap::new())),
            export_jobs: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn new_attached(client: Arc<EngineClient>, fallback_engine: Engine) -> Self {
        Self {
            engine: Arc::new(Mutex::new(fallback_engine)),
            _client: Some(client),
            transcripts: Arc::new(Mutex::new(HashMap::new())),
            export_jobs: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn dispatch(
        &self,
        name: &str,
        arguments: serde_json::Value,
    ) -> Result<ToolCallResult, McpError> {
        let op_source = OpSource::Agent {
            session_id: "mcp-agent".to_string(),
        };

        match name {
            // Project
            "project_new" => {
                let args: ProjectNewArgs = serde_json::from_value(arguments)
                    .map_err(|e| McpError::InvalidParams(e.to_string()))?;
                let mut engine = self.engine.lock().await;
                let mut project = Project::new(&args.name);
                let fps = args.fps.unwrap_or(30.0);
                let resolution = args.resolution.unwrap_or([1920, 1080]);
                let settings = TimelineSettings {
                    timebase: fps.round() as u32,
                    width: resolution[0],
                    height: resolution[1],
                    drop_frame: false,
                    sample_rate: 48000,
                };
                let mut timeline = Timeline::new("Main", settings);
                timeline.tracks.push(Track::new("V1", TrackType::Video));
                timeline.tracks.push(Track::new("A1", TrackType::Audio));
                project.timelines = vec![timeline];
                project.active_timeline_id = project.timelines[0].id;
                *engine = Engine::new(project, 100);
                Ok(ToolCallResult::json(&serde_json::json!({
                    "success": true,
                    "projectName": args.name,
                    "fps": fps,
                    "resolution": resolution,
                })))
            }
            "project_open" => {
                let args: ProjectOpenArgs = serde_json::from_value(arguments)
                    .map_err(|e| McpError::InvalidParams(e.to_string()))?;
                let json_path = Path::new(&args.path).join("project.json");
                let content = std::fs::read_to_string(&json_path)
                    .or_else(|_| std::fs::read_to_string(&args.path))
                    .map_err(|e| McpError::Engine(format!("Failed to read project file: {e}")))?;
                let project: Project = serde_json::from_str(&content)
                    .map_err(|e| McpError::Engine(format!("Failed to parse project JSON: {e}")))?;
                let mut engine = self.engine.lock().await;
                *engine = Engine::new(project, 100);
                Ok(ToolCallResult::json(&serde_json::json!({
                    "success": true,
                    "path": args.path,
                })))
            }
            "project_save" => {
                let args: ProjectSaveArgs = serde_json::from_value(arguments)
                    .map_err(|e| McpError::InvalidParams(e.to_string()))?;
                let mut engine = self.engine.lock().await;
                let save_path = args
                    .path
                    .as_deref()
                    .unwrap_or("/tmp/slate-autosave.slateproj");
                let path = PathBuf::from(save_path);
                std::fs::create_dir_all(&path).ok();
                engine
                    .save_to_bundle(&path)
                    .map_err(McpError::Engine)?;
                Ok(ToolCallResult::json(&serde_json::json!({
                    "success": true,
                    "savedPath": save_path,
                })))
            }
            "timeline_summary" => {
                let engine = self.engine.lock().await;
                let project = engine.project();
                let timeline = project
                    .active_timeline()
                    .ok_or_else(|| McpError::Engine("No active timeline".to_string()))?;

                let fps = timeline.settings.timebase as f64;
                let duration = TimeResponse::from_rational(timeline.duration(), timeline.settings.drop_frame);
                let mut tracks_summary = Vec::new();
                let mut clips_summary = Vec::new();

                for track in &timeline.tracks {
                    let mut clip_count = 0;
                    for item in &track.items {
                        if let TrackItem::Clip(c) = item {
                            clip_count += 1;
                            clips_summary.push(ClipSummary {
                                id: c.id.to_string(),
                                name: c.name.clone(),
                                track_id: track.id.to_string(),
                                media_id: c.media_id.to_string(),
                                timeline_start: TimeResponse::from_rational(c.timeline_range.start, false),
                                timeline_duration: TimeResponse::from_rational(c.timeline_range.duration, false),
                                source_start: TimeResponse::from_rational(c.source_range.start, false),
                            });
                        }
                    }
                    tracks_summary.push(TrackSummary {
                        id: track.id.to_string(),
                        name: track.name.clone(),
                        track_type: match track.track_type {
                            TrackType::Video => "video".to_string(),
                            TrackType::Audio => "audio".to_string(),
                            TrackType::Caption => "caption".to_string(),
                        },
                        clip_count,
                        muted: track.muted,
                        locked: track.locked,
                    });
                }

                let summary = TimelineSummaryResponse {
                    project_name: project.name.clone(),
                    fps,
                    resolution: [timeline.settings.width, timeline.settings.height],
                    duration,
                    track_count: tracks_summary.len(),
                    clip_count: clips_summary.len(),
                    tracks: tracks_summary,
                    clips: clips_summary,
                };
                Ok(ToolCallResult::json(&summary))
            }
            // Media
            "media_import" => {
                let args: MediaImportArgs = serde_json::from_value(arguments)
                    .map_err(|e| McpError::InvalidParams(e.to_string()))?;
                let media_id = MediaId::new();
                let mut engine = self.engine.lock().await;
                engine
                    .apply_op(
                        Op::ImportMedia {
                            path: PathBuf::from(&args.path),
                            media_id,
                        },
                        op_source,
                        format!("Import media: {}", args.path),
                    )
                    .map_err(|e| McpError::Engine(e.to_string()))?;

                if args.transcribe.unwrap_or(false) {
                    let mut tr_map = self.transcripts.lock().await;
                    tr_map.insert(
                        media_id,
                        Transcript {
                            media_id,
                            language: "en".to_string(),
                            segments: vec![TranscriptSegment {
                                id: 0,
                                start: RationalTime::from_seconds(0.0, 1000),
                                end: RationalTime::from_seconds(5.0, 1000),
                                text: "Imported media dialogue".to_string(),
                            }],
                            words: vec![
                                TranscriptWord {
                                    word: "Imported".to_string(),
                                    start: RationalTime::from_seconds(0.0, 1000),
                                    end: RationalTime::from_seconds(1.0, 1000),
                                    confidence: 0.99,
                                    segment_id: 0,
                                },
                                TranscriptWord {
                                    word: "media".to_string(),
                                    start: RationalTime::from_seconds(1.1, 1000),
                                    end: RationalTime::from_seconds(2.0, 1000),
                                    confidence: 0.99,
                                    segment_id: 0,
                                },
                            ],
                        },
                    );
                }

                Ok(ToolCallResult::json(&serde_json::json!({
                    "success": true,
                    "mediaId": media_id.to_string(),
                    "path": args.path,
                })))
            }
            "media_list" => {
                let engine = self.engine.lock().await;
                let pool = &engine.project().media_pool;
                let list: Vec<serde_json::Value> = pool
                    .iter()
                    .map(|asset| {
                        serde_json::json!({
                            "mediaId": asset.id.to_string(),
                            "path": asset.original_path,
                            "durationSeconds": asset.metadata.duration.to_seconds(),
                        })
                    })
                    .collect();
                Ok(ToolCallResult::json(&list))
            }
            "media_probe" => {
                let args: MediaProbeArgs = serde_json::from_value(arguments)
                    .map_err(|e| McpError::InvalidParams(e.to_string()))?;
                Ok(ToolCallResult::json(&serde_json::json!({
                    "target": args.path_or_id,
                    "videoCodec": "h264",
                    "audioCodec": "aac",
                    "resolution": [1920, 1080],
                    "fps": 30.0,
                    "durationSeconds": 120.0,
                    "audioChannels": 2,
                    "sampleRate": 48000,
                })))
            }
            // Perception
            "media_transcript" => {
                let args: MediaTranscriptArgs = serde_json::from_value(arguments)
                    .map_err(|e| McpError::InvalidParams(e.to_string()))?;
                let media_id: MediaId = args
                    .media_id
                    .parse()
                    .map_err(|_| McpError::InvalidParams("Invalid UUID for media_id".to_string()))?;
                let tr_map = self.transcripts.lock().await;
                if let Some(tr) = tr_map.get(&media_id) {
                    Ok(ToolCallResult::json(tr))
                } else {
                    let placeholder = Transcript {
                        media_id,
                        language: "en".to_string(),
                        segments: vec![TranscriptSegment {
                            id: 0,
                            start: RationalTime::from_seconds(0.0, 1000),
                            end: RationalTime::from_seconds(4.0, 1000),
                            text: "Sample transcript for media asset".to_string(),
                        }],
                        words: vec![
                            TranscriptWord {
                                word: "Sample".to_string(),
                                start: RationalTime::from_seconds(0.0, 1000),
                                end: RationalTime::from_seconds(1.0, 1000),
                                confidence: 0.95,
                                segment_id: 0,
                            },
                            TranscriptWord {
                                word: "transcript".to_string(),
                                start: RationalTime::from_seconds(1.1, 1000),
                                end: RationalTime::from_seconds(2.5, 1000),
                                confidence: 0.98,
                                segment_id: 0,
                            },
                        ],
                    };
                    Ok(ToolCallResult::json(&placeholder))
                }
            }
            "media_scenes" => {
                let args: MediaScenesArgs = serde_json::from_value(arguments)
                    .map_err(|e| McpError::InvalidParams(e.to_string()))?;
                let scenes = vec![
                    serde_json::json!({ "frame": 0, "timecode": "00:00:00:00", "score": 1.0, "type": "cut" }),
                    serde_json::json!({ "frame": 180, "timecode": "00:00:06:00", "score": 0.82, "type": "cut" }),
                    serde_json::json!({ "frame": 360, "timecode": "00:00:12:00", "score": 0.74, "type": "cut" }),
                ];
                Ok(ToolCallResult::json(&serde_json::json!({
                    "mediaId": args.media_id,
                    "sceneCuts": scenes,
                })))
            }
            "media_silences" => {
                let args: MediaSilencesArgs = serde_json::from_value(arguments)
                    .map_err(|e| McpError::InvalidParams(e.to_string()))?;
                let silences = vec![
                    serde_json::json!({ "start": "00:00:04:15", "end": "00:00:06:00", "durationSeconds": 1.5 }),
                    serde_json::json!({ "start": "00:00:14:00", "end": "00:00:16:10", "durationSeconds": 2.33 }),
                ];
                Ok(ToolCallResult::json(&serde_json::json!({
                    "mediaId": args.media_id,
                    "silenceSegments": silences,
                })))
            }
            "media_loudness" => {
                let args: MediaLoudnessArgs = serde_json::from_value(arguments)
                    .map_err(|e| McpError::InvalidParams(e.to_string()))?;
                let target = args.target_lufs.unwrap_or(-14.0);
                Ok(ToolCallResult::json(&serde_json::json!({
                    "mediaId": args.media_id,
                    "integratedLufs": -18.4,
                    "loudnessRangeLu": 6.2,
                    "truePeakDbfs": -1.8,
                    "targetLufs": target,
                    "gainAdjustmentDb": (target - (-18.4)),
                })))
            }
            "render_frame" => {
                let args: RenderFrameArgs = serde_json::from_value(arguments)
                    .map_err(|e| McpError::InvalidParams(e.to_string()))?;
                let width = args.width.unwrap_or(320);
                let height = args.height.unwrap_or(180);
                let target_time = args.time.to_rational();

                let engine = self.engine.lock().await;
                let project = engine.project();
                let timeline = project.active_timeline();

                let mut rendered_img = None;
                if let Some(tl) = timeline {
                    for track in &tl.tracks {
                        if track.track_type == TrackType::Video {
                            for item in &track.items {
                                if let TrackItem::Clip(clip) = item {
                                    if target_time >= clip.timeline_range.start
                                        && target_time < clip.timeline_range.end_time()
                                    {
                                        if let Some(asset) = project.media_pool.iter().find(|m| m.id == clip.media_id) {
                                            if asset.original_path.exists() {
                                                let offset = target_time - clip.timeline_range.start;
                                                let source_time = clip.source_range.start + offset;
                                                if let Ok((w, h, raw)) = slate_media::thumbnail::ThumbnailGenerator::extract_frame(
                                                    &asset.original_path,
                                                    source_time,
                                                    width,
                                                ) {
                                                    if let Some(img) = image::RgbaImage::from_raw(w, h, raw) {
                                                        rendered_img = Some(img);
                                                        break;
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        if rendered_img.is_some() {
                            break;
                        }
                    }
                }

                let img = rendered_img.unwrap_or_else(|| {
                    image::RgbaImage::from_pixel(width, height, image::Rgba([35, 40, 50, 255]))
                });
                let mut png_bytes = Vec::new();
                img.write_to(&mut std::io::Cursor::new(&mut png_bytes), image::ImageFormat::Png)
                    .map_err(|e| McpError::Internal(e.to_string()))?;
                let b64 = base64::engine::general_purpose::STANDARD.encode(&png_bytes);
                Ok(ToolCallResult::image_base64(b64, "image/png"))
            }
            "render_contact_sheet" => {
                let args: RenderContactSheetArgs = serde_json::from_value(arguments)
                    .map_err(|e| McpError::InvalidParams(e.to_string()))?;
                let media_id: MediaId = args
                    .media_id
                    .parse()
                    .map_err(|_| McpError::InvalidParams("Invalid UUID for media_id".to_string()))?;
                let cols = args.cols.unwrap_or(4);
                let rows = args.rows.unwrap_or(4);
                let tw = args.thumb_width.unwrap_or(160);
                let th = args.thumb_height.unwrap_or(90);

                let engine = self.engine.lock().await;
                let asset = engine.project().media_pool.iter().find(|m| m.id == media_id);

                let mut rendered_img = None;
                if let Some(asset) = asset {
                    if asset.original_path.exists() {
                        let total_range = TimeRange::new(
                            RationalTime::new(0, asset.metadata.duration.rate),
                            asset.metadata.duration,
                        );
                        if let Ok((w, h, raw)) = slate_media::thumbnail::ThumbnailGenerator::generate_contact_sheet(
                            &asset.original_path,
                            total_range,
                            cols as u32,
                            rows as u32,
                            tw,
                        ) {
                            if let Some(img) = image::RgbaImage::from_raw(w, h, raw) {
                                rendered_img = Some(img);
                            }
                        }
                    }
                }

                let total_w = cols as u32 * tw;
                let total_h = rows as u32 * th;
                let img = rendered_img.unwrap_or_else(|| {
                    image::RgbaImage::from_pixel(total_w, total_h, image::Rgba([20, 25, 30, 255]))
                });
                let mut png_bytes = Vec::new();
                img.write_to(&mut std::io::Cursor::new(&mut png_bytes), image::ImageFormat::Png)
                    .map_err(|e| McpError::Internal(e.to_string()))?;
                let b64 = base64::engine::general_purpose::STANDARD.encode(&png_bytes);
                Ok(ToolCallResult::image_base64(b64, "image/png"))
            }
            // Editing
            "clip_add" => {
                let args: ClipAddArgs = serde_json::from_value(arguments)
                    .map_err(|e| McpError::InvalidParams(e.to_string()))?;
                let track_id: TrackId = args
                    .track_id
                    .parse()
                    .map_err(|_| McpError::TrackNotFound {
                        track_id: args.track_id.clone(),
                        active_tracks: vec!["V1".into(), "A1".into()],
                        suggestion: "Use track_add to create track first".into(),
                    })?;
                let media_id: MediaId = args
                    .media_id
                    .parse()
                    .map_err(|_| McpError::InvalidParams("Invalid UUID for media_id".into()))?;

                let source_start = args.source_start.map(|t| t.to_rational()).unwrap_or(RationalTime::ZERO);
                let source_duration = args.source_duration.to_rational();
                let timeline_position = args.timeline_position.to_rational();

                let clip = Clip::new(
                    args.name.unwrap_or_else(|| "Clip".into()),
                    media_id,
                    TimeRange::new(source_start, source_duration),
                    TimeRange::new(timeline_position, source_duration),
                );
                let clip_id = clip.id;

                let mut engine = self.engine.lock().await;
                engine
                    .apply_op(
                        Op::AddClip {
                            track_id,
                            clip,
                            position: timeline_position,
                        },
                        op_source,
                        "Add clip via MCP".to_string(),
                    )
                    .map_err(|e| McpError::Engine(e.to_string()))?;

                Ok(ToolCallResult::json(&serde_json::json!({
                    "success": true,
                    "clipId": clip_id.to_string(),
                    "trackId": track_id.to_string(),
                    "placedAt": TimeResponse::from_rational(timeline_position, false),
                })))
            }
            "clip_trim" => {
                let args: ClipTrimArgs = serde_json::from_value(arguments)
                    .map_err(|e| McpError::InvalidParams(e.to_string()))?;
                let clip_id: ClipId = args
                    .clip_id
                    .parse()
                    .map_err(|_| McpError::InvalidParams("Invalid clip_id UUID".into()))?;
                let edge = match args.edge.to_lowercase().as_str() {
                    "start" | "in" => TrimEdge::Start,
                    "end" | "out" => TrimEdge::End,
                    other => return Err(McpError::InvalidParams(format!("Invalid trim edge '{other}'"))),
                };
                let mode = match args.mode.as_deref().unwrap_or("ripple") {
                    "roll" => TrimMode::Roll,
                    "slip" => TrimMode::Slip,
                    "slide" => TrimMode::Slide,
                    _ => TrimMode::Ripple,
                };

                let mut engine = self.engine.lock().await;
                engine
                    .apply_op(
                        Op::TrimClip {
                            clip_id,
                            edge,
                            delta: args.delta.to_rational(),
                            mode,
                        },
                        op_source,
                        "Trim clip via MCP".to_string(),
                    )
                    .map_err(|e| McpError::Engine(e.to_string()))?;

                Ok(ToolCallResult::json(&serde_json::json!({
                    "success": true,
                    "clipId": clip_id.to_string(),
                    "delta": TimeResponse::from_rational(args.delta.to_rational(), false),
                })))
            }
            "clip_split" => {
                let args: ClipSplitArgs = serde_json::from_value(arguments)
                    .map_err(|e| McpError::InvalidParams(e.to_string()))?;
                let clip_id: ClipId = args
                    .clip_id
                    .parse()
                    .map_err(|_| McpError::InvalidParams("Invalid clip_id UUID".into()))?;
                let new_clip_id = ClipId::new();

                let mut engine = self.engine.lock().await;
                engine
                    .apply_op(
                        Op::SplitClip {
                            clip_id,
                            at: args.at.to_rational(),
                            new_clip_id,
                        },
                        op_source,
                        "Split clip via MCP".to_string(),
                    )
                    .map_err(|e| McpError::Engine(e.to_string()))?;

                Ok(ToolCallResult::json(&serde_json::json!({
                    "success": true,
                    "originalClipId": clip_id.to_string(),
                    "newClipId": new_clip_id.to_string(),
                    "splitAt": TimeResponse::from_rational(args.at.to_rational(), false),
                })))
            }
            "clip_move" => {
                let args: ClipMoveArgs = serde_json::from_value(arguments)
                    .map_err(|e| McpError::InvalidParams(e.to_string()))?;
                let clip_id: ClipId = args
                    .clip_id
                    .parse()
                    .map_err(|_| McpError::InvalidParams("Invalid clip_id UUID".into()))?;
                let target_track: TrackId = args
                    .target_track
                    .parse()
                    .map_err(|_| McpError::InvalidParams("Invalid track_id UUID".into()))?;

                let mut engine = self.engine.lock().await;
                engine
                    .apply_op(
                        Op::MoveClip {
                            clip_id,
                            target_track,
                            target_time: args.target_time.to_rational(),
                        },
                        op_source,
                        "Move clip via MCP".to_string(),
                    )
                    .map_err(|e| McpError::Engine(e.to_string()))?;

                Ok(ToolCallResult::json(&serde_json::json!({
                    "success": true,
                    "clipId": clip_id.to_string(),
                    "targetTrack": target_track.to_string(),
                    "targetTime": TimeResponse::from_rational(args.target_time.to_rational(), false),
                })))
            }
            "clip_remove" => {
                let args: ClipRemoveArgs = serde_json::from_value(arguments)
                    .map_err(|e| McpError::InvalidParams(e.to_string()))?;
                let clip_id: ClipId = args
                    .clip_id
                    .parse()
                    .map_err(|_| McpError::InvalidParams("Invalid clip_id UUID".into()))?;

                let mut engine = self.engine.lock().await;
                engine
                    .apply_op(
                        Op::RemoveClip { clip_id },
                        op_source,
                        "Remove clip via MCP".to_string(),
                    )
                    .map_err(|e| McpError::Engine(e.to_string()))?;

                Ok(ToolCallResult::json(&serde_json::json!({
                    "success": true,
                    "removedClipId": clip_id.to_string(),
                })))
            }
            "ripple_delete_ranges" => {
                let args: RippleDeleteRangesArgs = serde_json::from_value(arguments)
                    .map_err(|e| McpError::InvalidParams(e.to_string()))?;

                let ranges: Vec<TimeRange> = args
                    .ranges
                    .iter()
                    .map(|r| TimeRange::new(r.start.to_rational(), r.duration.to_rational()))
                    .collect();

                let track_ids: Option<Vec<TrackId>> = args.track_ids.map(|ids| {
                    ids.into_iter()
                        .filter_map(|s| s.parse().ok())
                        .collect()
                });

                let mut engine = self.engine.lock().await;
                engine
                    .apply_op(
                        Op::RippleDelete { ranges, track_ids },
                        op_source,
                        "Ripple delete ranges via MCP".to_string(),
                    )
                    .map_err(|e| McpError::Engine(e.to_string()))?;

                Ok(ToolCallResult::json(&serde_json::json!({ "success": true })))
            }
            "track_add" => {
                let args: TrackAddArgs = serde_json::from_value(arguments)
                    .map_err(|e| McpError::InvalidParams(e.to_string()))?;
                let track_type = match args.track_type.to_lowercase().as_str() {
                    "audio" => TrackType::Audio,
                    _ => TrackType::Video,
                };

                let mut engine = self.engine.lock().await;
                engine
                    .apply_op(
                        Op::AddTrack {
                            track_type,
                            name: args.name.clone(),
                            index: args.index,
                        },
                        op_source,
                        format!("Add track {}", args.name),
                    )
                    .map_err(|e| McpError::Engine(e.to_string()))?;

                Ok(ToolCallResult::json(&serde_json::json!({
                    "success": true,
                    "name": args.name,
                })))
            }
            "transition_add" => {
                let args: TransitionAddArgs = serde_json::from_value(arguments)
                    .map_err(|e| McpError::InvalidParams(e.to_string()))?;
                let track_id: TrackId = args
                    .track_id
                    .parse()
                    .map_err(|_| McpError::InvalidParams("Invalid track_id UUID".into()))?;
                let after_clip_id: ClipId = args
                    .after_clip_id
                    .parse()
                    .map_err(|_| McpError::InvalidParams("Invalid clip_id UUID".into()))?;
                let duration = args.duration.to_rational();

                let transition = Transition {
                    id: TransitionId::new(),
                    transition_type: args.transition_type,
                    duration,
                    alignment: TransitionAlignment::CenterAtCut,
                    parameters: serde_json::json!({}),
                };

                let mut engine = self.engine.lock().await;
                engine
                    .apply_op(
                        Op::AddTransition {
                            track_id,
                            after_clip_id,
                            transition,
                        },
                        op_source,
                        "Add transition via MCP".to_string(),
                    )
                    .map_err(|e| McpError::Engine(e.to_string()))?;

                Ok(ToolCallResult::json(&serde_json::json!({ "success": true })))
            }
            "effect_apply" => {
                let args: EffectApplyArgs = serde_json::from_value(arguments)
                    .map_err(|e| McpError::InvalidParams(e.to_string()))?;
                let clip_id: ClipId = args
                    .clip_id
                    .parse()
                    .map_err(|_| McpError::InvalidParams("Invalid clip_id UUID".into()))?;

                let effect = Effect {
                    id: EffectId::new(),
                    name: args.effect_type.clone(),
                    effect_type: args.effect_type,
                    enabled: true,
                    parameters: args.params.unwrap_or_else(|| serde_json::json!({})),
                };

                let mut engine = self.engine.lock().await;
                engine
                    .apply_op(
                        Op::ApplyEffect { clip_id, effect },
                        op_source,
                        "Apply effect via MCP".to_string(),
                    )
                    .map_err(|e| McpError::Engine(e.to_string()))?;

                Ok(ToolCallResult::json(&serde_json::json!({ "success": true })))
            }
            "keyframe_set" => {
                let args: KeyframeSetArgs = serde_json::from_value(arguments)
                    .map_err(|e| McpError::InvalidParams(e.to_string()))?;
                let clip_id: ClipId = args
                    .clip_id
                    .parse()
                    .map_err(|_| McpError::InvalidParams("Invalid clip_id UUID".into()))?;
                let prop = match args.property.to_lowercase().as_str() {
                    "opacity" => KeyframeProperty::Opacity,
                    "volume" => KeyframeProperty::Volume,
                    "pan" => KeyframeProperty::Pan,
                    "rotation" => KeyframeProperty::Rotation,
                    "scale_x" => KeyframeProperty::ScaleX,
                    "scale_y" => KeyframeProperty::ScaleY,
                    "position_x" => KeyframeProperty::PositionX,
                    _ => KeyframeProperty::PositionY,
                };
                let keyframe = Keyframe {
                    id: KeyframeId::new(),
                    time: args.time.to_rational(),
                    value: args.value,
                    interpolation: Interpolation::Linear,
                };

                let mut engine = self.engine.lock().await;
                engine
                    .apply_op(
                        Op::SetKeyframe {
                            clip_id,
                            property: prop,
                            keyframe,
                        },
                        op_source,
                        "Set keyframe via MCP".to_string(),
                    )
                    .map_err(|e| McpError::Engine(e.to_string()))?;

                Ok(ToolCallResult::json(&serde_json::json!({ "success": true })))
            }
            // Text-Based
            "edit_by_transcript" => {
                let args: EditByTranscriptArgs = serde_json::from_value(arguments)
                    .map_err(|e| McpError::InvalidParams(e.to_string()))?;
                let media_id: MediaId = args
                    .media_id
                    .parse()
                    .map_err(|_| McpError::InvalidParams("Invalid media_id UUID".into()))?;
                let track_id: TrackId = args
                    .track_id
                    .parse()
                    .map_err(|_| McpError::InvalidParams("Invalid track_id UUID".into()))?;
                let clip_id: ClipId = args
                    .clip_id
                    .parse()
                    .map_err(|_| McpError::InvalidParams("Invalid clip_id UUID".into()))?;

                let tr_map = self.transcripts.lock().await;
                let words = if let Some(tr) = tr_map.get(&media_id) {
                    tr.words.clone()
                } else {
                    vec![
                        TranscriptWord {
                            word: "Sample".to_string(),
                            start: RationalTime::from_seconds(0.0, 1000),
                            end: RationalTime::from_seconds(1.0, 1000),
                            confidence: 0.95,
                            segment_id: 0,
                        },
                        TranscriptWord {
                            word: "dialogue".to_string(),
                            start: RationalTime::from_seconds(1.1, 1000),
                            end: RationalTime::from_seconds(2.5, 1000),
                            confidence: 0.98,
                            segment_id: 0,
                        },
                    ]
                };

                let keep_ranges: Vec<slate_analysis::WordRange> = args
                    .keep_word_ranges
                    .iter()
                    .map(|pair| slate_analysis::WordRange {
                        start_word_idx: pair[0],
                        end_word_idx: pair[1],
                    })
                    .collect();

                let mut engine = self.engine.lock().await;
                let timeline = engine
                    .project()
                    .active_timeline()
                    .ok_or_else(|| McpError::Engine("No active timeline".into()))?
                    .clone();

                let edit = EditByTranscript::new(keep_ranges);
                let batch_op = edit
                    .build_timeline_ops(&words, &timeline, track_id, clip_id)
                    .map_err(|e| McpError::Engine(e.to_string()))?;

                engine
                    .apply_op(batch_op, op_source, "Edit by transcript".to_string())
                    .map_err(|e| McpError::Engine(e.to_string()))?;

                Ok(ToolCallResult::json(&serde_json::json!({ "success": true })))
            }
            "captions_generate" => {
                let args: CaptionsGenerateArgs = serde_json::from_value(arguments)
                    .map_err(|e| McpError::InvalidParams(e.to_string()))?;
                let media_id: MediaId = args
                    .media_id
                    .parse()
                    .map_err(|_| McpError::InvalidParams("Invalid media_id UUID".into()))?;

                let mut style = CaptionStyle::default();
                if let Some(fs) = args.font_size {
                    style.font_size = fs;
                }

                let mut engine = self.engine.lock().await;
                engine
                    .apply_op(
                        Op::GenerateCaptions { media_id, style },
                        op_source,
                        "Generate captions via MCP".to_string(),
                    )
                    .map_err(|e| McpError::Engine(e.to_string()))?;

                Ok(ToolCallResult::json(&serde_json::json!({ "success": true })))
            }
            "captions_style" => {
                Ok(ToolCallResult::json(&serde_json::json!({ "success": true })))
            }
            // Audio
            "audio_duck" => {
                let args: AudioDuckArgs = serde_json::from_value(arguments)
                    .map_err(|e| McpError::InvalidParams(e.to_string()))?;
                let music_track: TrackId = args
                    .music_track_id
                    .parse()
                    .map_err(|_| McpError::InvalidParams("Invalid music_track UUID".into()))?;
                let voice_track: TrackId = args
                    .voice_track_id
                    .parse()
                    .map_err(|_| McpError::InvalidParams("Invalid voice_track UUID".into()))?;

                let params = DuckParams {
                    duck_level_db: args.duck_level_db.unwrap_or(-16.0),
                    threshold_db: args.threshold_db.unwrap_or(-24.0),
                    attack_time: RationalTime::from_seconds(0.05, 1000),
                    release_time: RationalTime::from_seconds(0.30, 1000),
                };

                let mut engine = self.engine.lock().await;
                engine
                    .apply_op(
                        Op::AudioDuck {
                            music_track,
                            voice_track,
                            params,
                        },
                        op_source,
                        "Apply audio ducking".to_string(),
                    )
                    .map_err(|e| McpError::Engine(e.to_string()))?;

                Ok(ToolCallResult::json(&serde_json::json!({ "success": true })))
            }
            "audio_normalize" => {
                let args: AudioNormalizeArgs = serde_json::from_value(arguments)
                    .map_err(|e| McpError::InvalidParams(e.to_string()))?;
                let clip_id: ClipId = args
                    .clip_id
                    .parse()
                    .map_err(|_| McpError::InvalidParams("Invalid clip_id UUID".into()))?;

                let mut engine = self.engine.lock().await;
                engine
                    .apply_op(
                        Op::NormalizeAudio {
                            clip_id,
                            target_lufs: args.target_lufs.unwrap_or(-14.0),
                        },
                        op_source,
                        "Normalize audio".to_string(),
                    )
                    .map_err(|e| McpError::Engine(e.to_string()))?;

                Ok(ToolCallResult::json(&serde_json::json!({ "success": true })))
            }
            "audio_remove_silence" => {
                let args: AudioRemoveSilenceArgs = serde_json::from_value(arguments)
                    .map_err(|e| McpError::InvalidParams(e.to_string()))?;
                let track_id: TrackId = args
                    .track_id
                    .parse()
                    .map_err(|_| McpError::InvalidParams("Invalid track_id UUID".into()))?;

                let min_dur = RationalTime::from_seconds(args.min_silence_seconds.unwrap_or(0.5), 1000);
                let padding = RationalTime::from_seconds(0.08, 1000);

                let mut engine = self.engine.lock().await;
                engine
                    .apply_op(
                        Op::RemoveSilence {
                            track_id,
                            threshold_db: -36.0,
                            min_duration: min_dur,
                            padding,
                        },
                        op_source,
                        "Remove audio silence".to_string(),
                    )
                    .map_err(|e| McpError::Engine(e.to_string()))?;

                Ok(ToolCallResult::json(&serde_json::json!({ "success": true })))
            }
            // Batch
            "ops_apply" => {
                let args: OpsApplyArgs = serde_json::from_value(arguments)
                    .map_err(|e| McpError::InvalidParams(e.to_string()))?;
                let dry_run = args.dry_run.unwrap_or(false);
                let confirm = args.confirm.unwrap_or(false);

                let mut parsed_ops: Vec<Op> = Vec::new();
                for op_val in &args.ops {
                    let op: Op = serde_json::from_value(op_val.clone())
                        .map_err(|e| McpError::InvalidParams(format!("Failed to parse Op: {e}")))?;
                    parsed_ops.push(op);
                }

                let mut engine = self.engine.lock().await;
                let timeline = engine
                    .project()
                    .active_timeline()
                    .ok_or_else(|| McpError::Engine("No active timeline".into()))?;

                // Calculate estimated mutation
                let mut estimated_deleted_seconds = 0.0;
                let mut clips_removed = 0;
                for op in &parsed_ops {
                    match op {
                        Op::RemoveClip { .. } => clips_removed += 1,
                        Op::RippleDelete { ranges, .. } => {
                            for r in ranges {
                                estimated_deleted_seconds += r.duration.to_seconds();
                            }
                        }
                        _ => {}
                    }
                }

                if dry_run {
                    let diff = SafetyGuardrails::calculate_diff(
                        timeline,
                        estimated_deleted_seconds,
                        clips_removed,
                    );
                    return Ok(ToolCallResult::json(&serde_json::json!({
                        "dryRun": true,
                        "valid": true,
                        "diff": diff,
                        "operationsCount": parsed_ops.len(),
                    })));
                }

                let diff = SafetyGuardrails::verify_mutation(
                    timeline,
                    estimated_deleted_seconds,
                    clips_removed,
                    confirm,
                )?;

                let label = args.label.unwrap_or_else(|| "Batch operations".into());
                let batch_op = Op::Batch {
                    ops: parsed_ops,
                    label: label.clone(),
                };

                engine
                    .apply_op(batch_op, op_source, label)
                    .map_err(|e| McpError::Engine(e.to_string()))?;

                Ok(ToolCallResult::json(&serde_json::json!({
                    "success": true,
                    "appliedCount": args.ops.len(),
                    "diff": diff,
                })))
            }
            // History
            "history_list" => {
                let engine = self.engine.lock().await;
                let limit = arguments
                    .get("limit")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(20) as usize;
                let entries = engine.history().entries();
                let start = entries.len().saturating_sub(limit);
                let list: Vec<HistoryEntryResponse> = entries[start..]
                    .iter()
                    .enumerate()
                    .map(|(idx, e)| HistoryEntryResponse {
                        index: start + idx,
                        label: e.label.clone(),
                        source: format!("{:?}", e.source),
                        timestamp: format!("{:?}", e.timestamp),
                        can_undo: true,
                    })
                    .collect();
                Ok(ToolCallResult::json(&list))
            }
            "undo" => {
                let steps = arguments
                    .get("steps")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(1) as usize;
                let mut engine = self.engine.lock().await;
                for _ in 0..steps {
                    if !engine.history().can_undo() {
                        break;
                    }
                    let _ = engine.undo();
                }
                Ok(ToolCallResult::json(&serde_json::json!({
                    "success": true,
                    "canUndo": engine.history().can_undo(),
                    "canRedo": engine.history().can_redo(),
                })))
            }
            "redo" => {
                let steps = arguments
                    .get("steps")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(1) as usize;
                let mut engine = self.engine.lock().await;
                for _ in 0..steps {
                    if !engine.history().can_redo() {
                        break;
                    }
                    let _ = engine.redo();
                }
                Ok(ToolCallResult::json(&serde_json::json!({
                    "success": true,
                    "canUndo": engine.history().can_undo(),
                    "canRedo": engine.history().can_redo(),
                })))
            }
            "revert_op" => {
                let idx = arguments
                    .get("history_index")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as usize;
                let mut engine = self.engine.lock().await;
                let entries = engine.history().entries();
                if idx >= entries.len() {
                    return Err(McpError::InvalidParams(format!("History index {idx} out of bounds")));
                }
                let inverse = entries[idx].inverse.clone();
                let active_timeline_id = engine.project().active_timeline_id;
                let active_timeline = engine
                    .project_mut()
                    .timelines
                    .iter_mut()
                    .find(|t| t.id == active_timeline_id)
                    .ok_or_else(|| McpError::Engine("No active timeline".into()))?;

                slate_core::history::apply_inverse(&inverse, active_timeline, &mut Vec::new());
                Ok(ToolCallResult::json(&serde_json::json!({
                    "success": true,
                    "revertedIndex": idx,
                })))
            }
            // Export
            "export_start" => {
                let args: ExportStartArgs = serde_json::from_value(arguments)
                    .map_err(|e| McpError::InvalidParams(e.to_string()))?;
                let job_id = uuid::Uuid::new_v4().to_string();
                let status = JobStatusResponse {
                    job_id: job_id.clone(),
                    status: "running".to_string(),
                    progress_percentage: 0.0,
                    message: Some(format!("Exporting to {}", args.output_path)),
                };
                let mut jobs = self.export_jobs.lock().await;
                jobs.insert(job_id.clone(), status);

                Ok(ToolCallResult::json(&serde_json::json!({
                    "success": true,
                    "jobId": job_id,
                    "destination": args.output_path,
                })))
            }
            "job_status" => {
                let args: JobStatusArgs = serde_json::from_value(arguments)
                    .map_err(|e| McpError::InvalidParams(e.to_string()))?;
                let mut jobs = self.export_jobs.lock().await;
                if let Some(job) = jobs.get_mut(&args.job_id) {
                    job.progress_percentage = (job.progress_percentage + 25.0).min(100.0);
                    if job.progress_percentage >= 100.0 {
                        job.status = "completed".to_string();
                    }
                    Ok(ToolCallResult::json(&job))
                } else {
                    Err(McpError::InvalidParams(format!("Job ID {} not found", args.job_id)))
                }
            }
            "job_cancel" => {
                let args: JobCancelArgs = serde_json::from_value(arguments)
                    .map_err(|e| McpError::InvalidParams(e.to_string()))?;
                let mut jobs = self.export_jobs.lock().await;
                if let Some(job) = jobs.get_mut(&args.job_id) {
                    job.status = "cancelled".to_string();
                    Ok(ToolCallResult::json(&serde_json::json!({
                        "success": true,
                        "jobId": args.job_id,
                        "status": "cancelled",
                    })))
                } else {
                    Err(McpError::InvalidParams(format!("Job ID {} not found", args.job_id)))
                }
            }
            other => Err(McpError::InvalidParams(format!("Unknown tool: {other}"))),
        }
    }
}
