use crate::error::IpcError;
use crate::protocol::requests::*;
use crate::protocol::responses::*;
use crate::protocol::rpc::EngineRpcServer;
use crate::server::state::ServerState;
use async_trait::async_trait;
use jsonrpsee::core::SubscriptionResult;
use jsonrpsee::PendingSubscriptionSink;
use slate_core::model::{TrackItem, TrackType};
use slate_core::RationalTime;

#[async_trait]
impl EngineRpcServer for ServerState {
    async fn apply_op(&self, params: ApplyOpParams) -> Result<ApplyOpResult, IpcError> {
        let mut engine = self.engine.lock().await;
        let label = params.label.clone();
        engine
            .apply_op(params.op, params.source, params.label)
            .map_err(|e| IpcError::Engine(e.to_string()))?;

        Ok(ApplyOpResult { ok: true, label })
    }

    async fn apply_batch(&self, params: ApplyBatchParams) -> Result<ApplyBatchResult, IpcError> {
        let mut engine = self.engine.lock().await;

        if params.dry_run {
            // Dry run test without committing
            return Ok(ApplyBatchResult {
                ok: true,
                applied_count: params.ops.len(),
                dry_run: true,
            });
        }

        let mut count = 0;
        for (i, op) in params.ops.into_iter().enumerate() {
            let label = format!("{} (sub-op #{})", params.label, i + 1);
            engine
                .apply_op(op, params.source.clone(), label)
                .map_err(|e| IpcError::Engine(e.to_string()))?;
            count += 1;
        }

        Ok(ApplyBatchResult {
            ok: true,
            applied_count: count,
            dry_run: false,
        })
    }

    async fn undo(&self, params: UndoParams) -> Result<UndoResult, IpcError> {
        let mut engine = self.engine.lock().await;
        let count = params.count.unwrap_or(1);
        let mut undone = 0;
        for _ in 0..count {
            if engine.undo().is_ok() {
                undone += 1;
            } else {
                break;
            }
        }
        Ok(UndoResult {
            ok: true,
            undone_count: undone,
        })
    }

    async fn redo(&self, params: RedoParams) -> Result<RedoResult, IpcError> {
        let mut engine = self.engine.lock().await;
        let count = params.count.unwrap_or(1);
        let mut redone = 0;
        for _ in 0..count {
            if engine.redo().is_ok() {
                redone += 1;
            } else {
                break;
            }
        }
        Ok(RedoResult {
            ok: true,
            redone_count: redone,
        })
    }

    async fn revert(&self, params: RevertParams) -> Result<RevertResult, IpcError> {
        let mut engine = self.engine.lock().await;
        let inverse = {
            let history = engine.history();
            if params.op_index >= history.entries().len() {
                return Err(IpcError::Engine(format!(
                    "History index {} out of bounds",
                    params.op_index
                )));
            }
            history.entries()[params.op_index].inverse.clone()
        };

        let proj = engine.project_mut();
        let tid = proj.active_timeline_id;
        let timeline = proj
            .timelines
            .iter_mut()
            .find(|t| t.id == tid)
            .ok_or_else(|| IpcError::Engine("Active timeline not found".into()))?;

        slate_core::history::apply_inverse(&inverse, timeline, &mut proj.media_pool);
        Ok(RevertResult { ok: true })
    }

    async fn list_history(&self, params: ListHistoryParams) -> Result<HistoryListResult, IpcError> {
        let engine = self.engine.lock().await;
        let history = engine.history();
        let entries = if let Some(limit) = params.limit {
            history.entries().iter().rev().take(limit).cloned().collect()
        } else {
            history.entries().to_vec()
        };

        Ok(HistoryListResult {
            can_undo: history.can_undo(),
            can_redo: history.can_redo(),
            entries,
        })
    }

    async fn get_timeline(&self, params: GetTimelineParams) -> Result<TimelineResult, IpcError> {
        let engine = self.engine.lock().await;
        let proj = engine.project();
        let tid = params.timeline_id.unwrap_or(proj.active_timeline_id);
        let timeline = proj
            .timelines
            .iter()
            .find(|t| t.id == tid)
            .cloned()
            .ok_or_else(|| IpcError::Engine(format!("Timeline with ID {:?} not found", tid)))?;

        Ok(TimelineResult { timeline })
    }

    async fn get_timeline_summary(
        &self,
        params: GetSummaryParams,
    ) -> Result<TimelineSummaryResult, IpcError> {
        let engine = self.engine.lock().await;
        let proj = engine.project();
        let tid = params.timeline_id.unwrap_or(proj.active_timeline_id);
        let timeline = proj
            .timelines
            .iter()
            .find(|t| t.id == tid)
            .ok_or_else(|| IpcError::Engine(format!("Timeline with ID {:?} not found", tid)))?;

        let mut tracks = Vec::new();
        for t in &timeline.tracks {
            let mut clips = Vec::new();
            for item in &t.items {
                if let TrackItem::Clip(c) = item {
                    clips.push(TimelineClipSummary {
                        clip_id: c.id.to_string(),
                        name: c.name.clone(),
                        start: c.timeline_range.start,
                        duration: c.timeline_range.duration,
                    });
                }
            }
            tracks.push(TimelineTrackSummary {
                track_id: t.id.to_string(),
                name: t.name.clone(),
                track_type: match t.track_type {
                    TrackType::Video => "video".into(),
                    TrackType::Audio => "audio".into(),
                    TrackType::Caption => "caption".into(),
                },
                clips,
            });
        }

        Ok(TimelineSummaryResult {
            timeline_id: timeline.id,
            name: timeline.name.clone(),
            duration: timeline.duration(),
            tracks,
        })
    }

    async fn list_media(&self) -> Result<MediaListResult, IpcError> {
        let engine = self.engine.lock().await;
        Ok(MediaListResult {
            items: engine.project().media_pool.clone(),
        })
    }

    async fn probe_media(&self, params: ProbeMediaParams) -> Result<ProbeMediaResult, IpcError> {
        let meta = slate_media::MediaProbe::probe(&params.path)
            .map_err(|e| IpcError::Engine(e.to_string()))?;

        Ok(ProbeMediaResult { metadata: meta })
    }

    async fn import_media(&self, params: ImportMediaParams) -> Result<ImportMediaResult, IpcError> {
        let mut engine = self.engine.lock().await;
        let media_id = slate_core::MediaId::new();

        let meta = slate_core::model::MediaMetadata {
            file_size_bytes: std::fs::metadata(&params.path).map(|m| m.len()).unwrap_or(0),
            duration: RationalTime::new(10, 1),
            video_codec: Some("h264".into()),
            width: Some(1920),
            height: Some(1080),
            frame_rate: Some(30.0),
            timebase: 30,
            audio_codec: Some("aac".into()),
            audio_channels: Some(2),
            sample_rate: Some(48000),
        };

        let media_ref = slate_core::model::MediaReference {
            id: media_id,
            original_path: params.path.clone(),
            relative_path: params.path.clone(),
            metadata: meta,
            proxy_path: None,
            waveform_path: None,
        };

        engine.project_mut().media_pool.push(media_ref.clone());

        Ok(ImportMediaResult {
            media_id,
            media_ref,
        })
    }

    async fn playback_play(&self, params: PlayParams) -> Result<PlaybackResult, IpcError> {
        Ok(PlaybackResult {
            is_playing: true,
            speed: params.speed.unwrap_or(1.0),
            position: RationalTime::ZERO,
        })
    }

    async fn playback_pause(&self) -> Result<PlaybackResult, IpcError> {
        Ok(PlaybackResult {
            is_playing: false,
            speed: 0.0,
            position: RationalTime::ZERO,
        })
    }

    async fn playback_seek(&self, params: SeekParams) -> Result<PlaybackResult, IpcError> {
        Ok(PlaybackResult {
            is_playing: false,
            speed: 0.0,
            position: params.time,
        })
    }

    async fn subscribe_events(&self, pending: PendingSubscriptionSink) -> SubscriptionResult {
        let sink = pending.accept().await?;
        let mut rx = self.engine.lock().await.subscribe();
        tokio::spawn(async move {
            while let Ok(event) = rx.recv().await {
                let msg = jsonrpsee::SubscriptionMessage::from_json(&event);
                match msg {
                    Ok(m) => {
                        if sink.send(m).await.is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });
        Ok(())
    }
}
