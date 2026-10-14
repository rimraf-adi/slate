use crate::model::MediaReference;
use std::path::PathBuf;

pub struct PathResolver {
    bundle_dir: PathBuf,
    search_paths: Vec<PathBuf>,
}

impl PathResolver {
    pub fn new(bundle_dir: PathBuf, search_paths: Vec<PathBuf>) -> Self {
        Self {
            bundle_dir,
            search_paths,
        }
    }

    pub fn resolve(&self, reference: &MediaReference) -> Option<PathBuf> {
        // 1. Try relative path from bundle
        let rel_target = self.bundle_dir.join(&reference.relative_path);
        if rel_target.exists() {
            return Some(rel_target);
        }

        // 2. Try original absolute path
        if reference.original_path.exists() {
            return Some(reference.original_path.clone());
        }

        // 3. Search user-configured asset directories by filename
        if let Some(file_name) = reference.original_path.file_name() {
            for search_root in &self.search_paths {
                let candidate = search_root.join(file_name);
                if candidate.exists() {
                    return Some(candidate);
                }
            }
        }

        None
    }
}
