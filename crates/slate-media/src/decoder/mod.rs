pub mod worker;

use crate::probe::MediaProbe;
use parking_lot::Mutex;
use slate_core::model::MediaMetadata;
use slate_core::time::{RationalTime, TimeRange};
use slate_core::traits::decoder::{AudioBuffer, DecoderHandle, MediaDecoder, MediaError, VideoFrame};
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use worker::FileDecoder;

pub struct FfmpegMediaDecoder {
    next_handle_id: AtomicU64,
    decoders: Arc<Mutex<HashMap<u64, FileDecoder>>>,
}

impl FfmpegMediaDecoder {
    pub fn new() -> Self {
        Self {
            next_handle_id: AtomicU64::new(1),
            decoders: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl Default for FfmpegMediaDecoder {
    fn default() -> Self {
        Self::new()
    }
}

impl MediaDecoder for FfmpegMediaDecoder {
    fn probe(&self, path: &Path) -> Result<MediaMetadata, MediaError> {
        MediaProbe::probe(path).map_err(|e| MediaError::ProbeFailed(path.to_path_buf(), e.to_string()))
    }

    fn open(&self, path: &Path) -> Result<DecoderHandle, MediaError> {
        let decoder = FileDecoder::open(path)
            .map_err(|e| MediaError::ProbeFailed(path.to_path_buf(), e.to_string()))?;

        let handle_id = self.next_handle_id.fetch_add(1, Ordering::SeqCst);
        self.decoders.lock().insert(handle_id, decoder);

        Ok(DecoderHandle(handle_id))
    }

    fn decode_frame(
        &self,
        handle: &DecoderHandle,
        time: RationalTime,
    ) -> Result<VideoFrame, MediaError> {
        let mut decoders = self.decoders.lock();
        let decoder = decoders
            .get_mut(&handle.0)
            .ok_or(MediaError::SeekError(time))?;

        decoder
            .decode_video_frame(time)
            .map_err(|_| MediaError::SeekError(time))
    }

    fn decode_audio(
        &self,
        handle: &DecoderHandle,
        range: TimeRange,
    ) -> Result<AudioBuffer, MediaError> {
        let mut decoders = self.decoders.lock();
        let decoder = decoders
            .get_mut(&handle.0)
            .ok_or(MediaError::SeekError(range.start))?;

        decoder
            .decode_audio_range(range)
            .map_err(|_| MediaError::SeekError(range.start))
    }

    fn close(&self, handle: DecoderHandle) -> Result<(), MediaError> {
        self.decoders.lock().remove(&handle.0);
        Ok(())
    }
}
