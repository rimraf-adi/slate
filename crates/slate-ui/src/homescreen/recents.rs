//! Recent projects persistence and management.

use std::fs;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecentProject {
    pub id: String,
    pub name: String,
    pub path: PathBuf,
    pub last_opened_epoch_secs: u64,
    pub duration_seconds: f64,
    pub resolution: [u32; 2],
    pub fps: f64,
    pub track_count: usize,
    pub clip_count: usize,
}

impl RecentProject {
    pub fn new(
        name: impl Into<String>,
        path: PathBuf,
        duration_seconds: f64,
        resolution: [u32; 2],
        fps: f64,
        track_count: usize,
        clip_count: usize,
    ) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            path,
            last_opened_epoch_secs: now,
            duration_seconds,
            resolution,
            fps,
            track_count,
            clip_count,
        }
    }

    pub fn formatted_time_ago(&self) -> String {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        if now <= self.last_opened_epoch_secs {
            return "Just now".to_string();
        }

        let diff = now - self.last_opened_epoch_secs;
        if diff < 60 {
            "Just now".to_string()
        } else if diff < 3600 {
            format!("{}m ago", diff / 60)
        } else if diff < 86400 {
            format!("{}h ago", diff / 3600)
        } else {
            format!("{}d ago", diff / 86400)
        }
    }

    pub fn formatted_duration(&self) -> String {
        let secs = self.duration_seconds.round() as u64;
        let mins = secs / 60;
        let rem_secs = secs % 60;
        format!("{:02}:{:02}", mins, rem_secs)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentsStore {
    pub projects: Vec<RecentProject>,
}

impl Default for RecentsStore {
    fn default() -> Self {
        Self::with_samples()
    }
}

impl RecentsStore {
    pub fn storage_path() -> PathBuf {
        if let Ok(home) = std::env::var("HOME") {
            let p = PathBuf::from(home).join(".config").join("slate").join("recents.json");
            if let Some(parent) = p.parent() {
                let _ = fs::create_dir_all(parent);
            }
            return p;
        }
        PathBuf::from("/tmp/slate_recents.json")
    }

    pub fn with_samples() -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        Self {
            projects: vec![
                RecentProject {
                    id: "sample-commercial".into(),
                    name: "Commercial Brand Campaign".into(),
                    path: PathBuf::from("/tmp/slate_samples/commercial.slateproj"),
                    last_opened_epoch_secs: now.saturating_sub(1800),
                    duration_seconds: 45.0,
                    resolution: [1920, 1080],
                    fps: 30.0,
                    track_count: 4,
                    clip_count: 8,
                },
                RecentProject {
                    id: "sample-vlog".into(),
                    name: "Tokyo Travel Vlog 4K".into(),
                    path: PathBuf::from("/tmp/slate_samples/tokyo_vlog.slateproj"),
                    last_opened_epoch_secs: now.saturating_sub(7200),
                    duration_seconds: 180.0,
                    resolution: [3840, 2160],
                    fps: 24.0,
                    track_count: 6,
                    clip_count: 14,
                },
                RecentProject {
                    id: "sample-shorts".into(),
                    name: "Viral Tech Shorts (9:16)".into(),
                    path: PathBuf::from("/tmp/slate_samples/tech_shorts.slateproj"),
                    last_opened_epoch_secs: now.saturating_sub(86400),
                    duration_seconds: 30.0,
                    resolution: [1080, 1920],
                    fps: 60.0,
                    track_count: 3,
                    clip_count: 6,
                },
                RecentProject {
                    id: "sample-podcast".into(),
                    name: "Founder Podcast Ep 42".into(),
                    path: PathBuf::from("/tmp/slate_samples/founder_pod.slateproj"),
                    last_opened_epoch_secs: now.saturating_sub(172800),
                    duration_seconds: 1240.0,
                    resolution: [1920, 1080],
                    fps: 30.0,
                    track_count: 5,
                    clip_count: 12,
                },
            ],
        }
    }

    pub fn load() -> Self {
        let path = Self::storage_path();
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(store) = serde_json::from_str::<RecentsStore>(&content) {
                    if !store.projects.is_empty() {
                        return store;
                    }
                }
            }
        }
        let sample = Self::with_samples();
        sample.save();
        sample
    }

    pub fn save(&self) {
        let path = Self::storage_path();
        if let Ok(content) = serde_json::to_string_pretty(self) {
            let _ = fs::write(path, content);
        }
    }

    pub fn add(&mut self, mut item: RecentProject) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        item.last_opened_epoch_secs = now;

        // Remove if duplicate path
        self.projects.retain(|p| p.path != item.path);
        // Insert at beginning
        self.projects.insert(0, item);
        // Keep top 20
        if self.projects.len() > 20 {
            self.projects.truncate(20);
        }
        self.save();
    }

    pub fn remove(&mut self, path: &Path) {
        self.projects.retain(|p| p.path != path);
        self.save();
    }
}
