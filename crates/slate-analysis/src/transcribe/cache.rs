use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

use crate::error::AnalysisError;
use crate::types::{Transcript, TranscriptionParams};

pub struct TranscriptCache {
    cache_dir: PathBuf,
}

impl TranscriptCache {
    pub fn new(project_cache_root: &Path) -> Self {
        let dir = project_cache_root.join("transcripts");
        let _ = fs::create_dir_all(&dir);
        Self { cache_dir: dir }
    }

    pub fn compute_key(media_sha256: &str, params: &TranscriptionParams) -> String {
        let mut hasher = Sha256::new();
        hasher.update(media_sha256.as_bytes());
        hasher.update(format!("{:?}", params.model_size).as_bytes());
        if let Some(ref lang) = params.language {
            hasher.update(lang.as_bytes());
        }
        hasher.update([params.translate_to_english as u8]);
        hasher.update([params.word_timestamps as u8]);
        hex::encode(hasher.finalize())
    }

    pub fn get(&self, cache_key: &str) -> Option<Transcript> {
        let path = self.cache_dir.join(format!("{cache_key}.json"));
        if !path.exists() {
            return None;
        }
        let data = fs::read_to_string(path).ok()?;
        serde_json::from_str(&data).ok()
    }

    pub fn put(&self, cache_key: &str, transcript: &Transcript) -> Result<(), AnalysisError> {
        let final_path = self.cache_dir.join(format!("{cache_key}.json"));
        let temp_path = self
            .cache_dir
            .join(format!("{cache_key}.tmp.{}", uuid::Uuid::new_v4()));

        let serialized = serde_json::to_string_pretty(transcript)
            .map_err(|e| AnalysisError::Serialization(e.to_string()))?;

        fs::write(&temp_path, serialized)?;
        fs::rename(&temp_path, &final_path)?;

        Ok(())
    }
}
