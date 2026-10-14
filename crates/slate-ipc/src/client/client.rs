use crate::client::discovery::discover_and_connect;
use crate::error::IpcError;
use crate::protocol::events::EngineEvent;
use crate::protocol::requests::*;
use crate::protocol::responses::*;
use crate::protocol::rpc::EngineRpcClient;
use jsonrpsee::core::client::Subscription;
use jsonrpsee::ws_client::WsClient;
use slate_core::history::OpSource;
use slate_core::id::TimelineId;
use slate_core::op::Op;
use slate_core::time::RationalTime;
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Clone)]
pub struct EngineClient {
    client: std::sync::Arc<WsClient>,
}

impl EngineClient {
    /// Discovers and connects to a running Slate editor instance for the specified project.
    pub async fn connect_project(
        project_path: impl AsRef<Path>,
        timeout: Duration,
    ) -> Result<Self, IpcError> {
        let client = discover_and_connect(project_path.as_ref(), timeout).await?;
        Ok(Self { client: std::sync::Arc::new(client) })
    }

    /// Directly connects to a given WebSocket URL (e.g. `ws://127.0.0.1:9090`).
    pub async fn connect_url(url: &str) -> Result<Self, IpcError> {
        let client = jsonrpsee::ws_client::WsClientBuilder::default()
            .build(url)
            .await
            .map_err(|e| IpcError::Engine(format!("Failed to connect to {}: {}", url, e)))?;
        Ok(Self { client: std::sync::Arc::new(client) })
    }

    pub async fn apply_op(
        &self,
        op: Op,
        source: OpSource,
        label: impl Into<String>,
    ) -> Result<ApplyOpResult, IpcError> {
        self.client
            .apply_op(ApplyOpParams {
                op,
                source,
                label: label.into(),
            })
            .await
            .map_err(|e| IpcError::Engine(e.to_string()))
    }

    pub async fn apply_batch(
        &self,
        ops: Vec<Op>,
        source: OpSource,
        label: impl Into<String>,
        dry_run: bool,
    ) -> Result<ApplyBatchResult, IpcError> {
        self.client
            .apply_batch(ApplyBatchParams {
                ops,
                source,
                label: label.into(),
                dry_run,
            })
            .await
            .map_err(|e| IpcError::Engine(e.to_string()))
    }

    pub async fn undo(&self, count: Option<usize>) -> Result<UndoResult, IpcError> {
        self.client
            .undo(UndoParams { count })
            .await
            .map_err(|e| IpcError::Engine(e.to_string()))
    }

    pub async fn redo(&self, count: Option<usize>) -> Result<RedoResult, IpcError> {
        self.client
            .redo(RedoParams { count })
            .await
            .map_err(|e| IpcError::Engine(e.to_string()))
    }

    pub async fn revert(&self, op_index: usize) -> Result<RevertResult, IpcError> {
        self.client
            .revert(RevertParams { op_index })
            .await
            .map_err(|e| IpcError::Engine(e.to_string()))
    }

    pub async fn list_history(&self, limit: Option<usize>) -> Result<HistoryListResult, IpcError> {
        self.client
            .list_history(ListHistoryParams { limit })
            .await
            .map_err(|e| IpcError::Engine(e.to_string()))
    }

    pub async fn get_timeline(&self, timeline_id: Option<TimelineId>) -> Result<TimelineResult, IpcError> {
        self.client
            .get_timeline(GetTimelineParams { timeline_id })
            .await
            .map_err(|e| IpcError::Engine(e.to_string()))
    }

    pub async fn get_timeline_summary(
        &self,
        timeline_id: Option<TimelineId>,
    ) -> Result<TimelineSummaryResult, IpcError> {
        self.client
            .get_timeline_summary(GetSummaryParams { timeline_id })
            .await
            .map_err(|e| IpcError::Engine(e.to_string()))
    }

    pub async fn list_media(&self) -> Result<MediaListResult, IpcError> {
        self.client
            .list_media()
            .await
            .map_err(|e| IpcError::Engine(e.to_string()))
    }

    pub async fn probe_media(&self, path: PathBuf) -> Result<ProbeMediaResult, IpcError> {
        self.client
            .probe_media(ProbeMediaParams { path })
            .await
            .map_err(|e| IpcError::Engine(e.to_string()))
    }

    pub async fn import_media(
        &self,
        path: PathBuf,
        generate_proxy: bool,
        generate_waveform: bool,
    ) -> Result<ImportMediaResult, IpcError> {
        self.client
            .import_media(ImportMediaParams {
                path,
                generate_proxy,
                generate_waveform,
            })
            .await
            .map_err(|e| IpcError::Engine(e.to_string()))
    }

    pub async fn playback_play(&self, speed: Option<f64>) -> Result<PlaybackResult, IpcError> {
        self.client
            .playback_play(PlayParams { speed })
            .await
            .map_err(|e| IpcError::Engine(e.to_string()))
    }

    pub async fn playback_pause(&self) -> Result<PlaybackResult, IpcError> {
        self.client
            .playback_pause()
            .await
            .map_err(|e| IpcError::Engine(e.to_string()))
    }

    pub async fn playback_seek(&self, time: RationalTime) -> Result<PlaybackResult, IpcError> {
        self.client
            .playback_seek(SeekParams { time })
            .await
            .map_err(|e| IpcError::Engine(e.to_string()))
    }

    pub async fn subscribe_events(&self) -> Result<Subscription<EngineEvent>, IpcError> {
        self.client
            .subscribe_events()
            .await
            .map_err(|e| IpcError::Engine(e.to_string()))
    }
}
