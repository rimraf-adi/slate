use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Handles creation and directory layout verification for `.slateproj` bundles.
pub struct ProjectBundle;

impl ProjectBundle {
    pub const PROJECT_JSON: &'static str = "project.json";
    pub const AUTOSAVE_JOURNAL: &'static str = "autosave.journal";
    pub const CACHE_DIR: &'static str = "cache";
    pub const PROXIES_DIR: &'static str = "cache/proxies";
    pub const WAVEFORMS_DIR: &'static str = "cache/waveforms";
    pub const THUMBNAILS_DIR: &'static str = "cache/thumbnails";
    pub const TRANSCRIPTS_DIR: &'static str = "cache/transcripts";
    pub const EXPORTS_DIR: &'static str = "exports";

    /// Initializes a brand new directory bundle structure on disk.
    pub fn init_bundle(path: &Path) -> io::Result<()> {
        fs::create_dir_all(path.join(Self::PROXIES_DIR))?;
        fs::create_dir_all(path.join(Self::WAVEFORMS_DIR))?;
        fs::create_dir_all(path.join(Self::THUMBNAILS_DIR))?;
        fs::create_dir_all(path.join(Self::TRANSCRIPTS_DIR))?;
        fs::create_dir_all(path.join(Self::EXPORTS_DIR))?;
        Ok(())
    }

    /// Returns the canonical path to project.json inside the bundle.
    pub fn project_json_path(bundle_dir: &Path) -> PathBuf {
        bundle_dir.join(Self::PROJECT_JSON)
    }

    /// Returns the canonical path to autosave.journal inside the bundle.
    pub fn journal_path(bundle_dir: &Path) -> PathBuf {
        bundle_dir.join(Self::AUTOSAVE_JOURNAL)
    }

    /// Saves project state atomically into the bundle directory.
    pub fn save_to_bundle(bundle_dir: &Path, project: &crate::model::Project) -> io::Result<()> {
        Self::init_bundle(bundle_dir)?;
        let json_path = Self::project_json_path(bundle_dir);
        let tmp_path = bundle_dir.join("project.json.tmp");

        let serialized = serde_json::to_string_pretty(project)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        fs::write(&tmp_path, serialized)?;
        fs::rename(&tmp_path, &json_path)?;
        Ok(())
    }

    /// Loads project state from a bundle directory.
    pub fn load_from_bundle(bundle_dir: &Path) -> io::Result<crate::model::Project> {
        let json_path = Self::project_json_path(bundle_dir);
        let data = fs::read_to_string(&json_path)?;
        let project = serde_json::from_str(&data)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        Ok(project)
    }
}
