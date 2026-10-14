use crate::model::Project;
use crate::op::{Op, OpApplicable};
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

pub struct JournalWriter {
    file: File,
}

impl JournalWriter {
    pub fn open(bundle_dir: &Path) -> std::io::Result<Self> {
        let path = bundle_dir.join("autosave.journal");
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        Ok(Self { file })
    }

    pub fn append_op(&mut self, op: &Op) -> std::io::Result<()> {
        let serialized = serde_json::to_string(op)?;
        let checksum = crc32fast::hash(serialized.as_bytes());
        writeln!(self.file, "{:08x}:{}", checksum, serialized)?;
        self.file.sync_data()?;
        Ok(())
    }

    pub fn truncate(&mut self) -> std::io::Result<()> {
        self.file.set_len(0)?;
        self.file.sync_all()?;
        Ok(())
    }
}

pub struct JournalRecovery;

impl JournalRecovery {
    pub fn recover(bundle_dir: &Path, project: &mut Project) -> Result<usize, String> {
        let path = bundle_dir.join("autosave.journal");
        if !path.exists() {
            return Ok(0);
        }

        let file = File::open(&path).map_err(|e| e.to_string())?;
        let reader = BufReader::new(file);
        let mut recovered_count = 0;

        let active_timeline = project
            .timelines
            .iter_mut()
            .find(|t| t.id == project.active_timeline_id)
            .ok_or_else(|| "Active timeline not found in project".to_string())?;

        for (line_idx, line_res) in reader.lines().enumerate() {
            let line = line_res.map_err(|e| e.to_string())?;
            if line.trim().is_empty() {
                continue;
            }

            let parts: Vec<&str> = line.splitn(2, ':').collect();
            if parts.len() != 2 {
                return Err(format!("Corrupt journal entry at line {}", line_idx + 1));
            }

            let expected_crc = u32::from_str_radix(parts[0], 16)
                .map_err(|_| format!("Invalid CRC hex at line {}", line_idx + 1))?;
            let actual_crc = crc32fast::hash(parts[1].as_bytes());

            if expected_crc != actual_crc {
                return Err(format!("CRC mismatch at journal line {}", line_idx + 1));
            }

            let op: Op = serde_json::from_str(parts[1])
                .map_err(|e| format!("Failed to deserialize op at line {}: {}", line_idx + 1, e))?;

            op.apply(active_timeline, &mut project.media_pool)
                .map_err(|e| format!("Failed to replay journal op at line {}: {:?}", line_idx + 1, e))?;

            recovered_count += 1;
        }

        Ok(recovered_count)
    }
}
