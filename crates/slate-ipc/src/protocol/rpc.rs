use crate::error::IpcError;
use crate::protocol::events::EngineEvent;
use crate::protocol::requests::*;
use crate::protocol::responses::*;
use jsonrpsee::core::SubscriptionResult;
use jsonrpsee::proc_macros::rpc;

#[rpc(server, client, namespace = "")]
pub trait EngineRpc {
    #[method(name = "op.apply")]
    async fn apply_op(&self, params: ApplyOpParams) -> Result<ApplyOpResult, IpcError>;

    #[method(name = "op.batch")]
    async fn apply_batch(&self, params: ApplyBatchParams) -> Result<ApplyBatchResult, IpcError>;

    #[method(name = "op.undo")]
    async fn undo(&self, params: UndoParams) -> Result<UndoResult, IpcError>;

    #[method(name = "op.redo")]
    async fn redo(&self, params: RedoParams) -> Result<RedoResult, IpcError>;

    #[method(name = "op.revert")]
    async fn revert(&self, params: RevertParams) -> Result<RevertResult, IpcError>;

    #[method(name = "history.list")]
    async fn list_history(&self, params: ListHistoryParams) -> Result<HistoryListResult, IpcError>;

    #[method(name = "timeline.get")]
    async fn get_timeline(&self, params: GetTimelineParams) -> Result<TimelineResult, IpcError>;

    #[method(name = "timeline.summary")]
    async fn get_timeline_summary(&self, params: GetSummaryParams) -> Result<TimelineSummaryResult, IpcError>;

    #[method(name = "media.list")]
    async fn list_media(&self) -> Result<MediaListResult, IpcError>;

    #[method(name = "media.probe")]
    async fn probe_media(&self, params: ProbeMediaParams) -> Result<ProbeMediaResult, IpcError>;

    #[method(name = "media.import")]
    async fn import_media(&self, params: ImportMediaParams) -> Result<ImportMediaResult, IpcError>;

    #[method(name = "playback.play")]
    async fn playback_play(&self, params: PlayParams) -> Result<PlaybackResult, IpcError>;

    #[method(name = "playback.pause")]
    async fn playback_pause(&self) -> Result<PlaybackResult, IpcError>;

    #[method(name = "playback.seek")]
    async fn playback_seek(&self, params: SeekParams) -> Result<PlaybackResult, IpcError>;

    #[subscription(name = "events.subscribe", unsubscribe = "events.unsubscribe", item = EngineEvent)]
    async fn subscribe_events(&self) -> SubscriptionResult;
}
