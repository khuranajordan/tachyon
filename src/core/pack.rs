use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::process::Command;
use byteorder::{BigEndian, ReadBytesExt};
use flate2::read::ZlibDecoder;
use super::hash::Oid;
use super::object::{GitObject, ObjectType};

pub struct PackManager {
    git_dir: PathBuf,
}

impl PackManager {
    pub fn new(git_dir: &Path) -> Self {
        Self {
            git_dir: git_dir.to_path_buf(),
        }
    }

    pub fn read_object(&self, oid: &Oid) -> Result<GitObject, String> {
        // 1. Try reading directly from packfiles
        let pack_dir = self.git_dir.join("objects").join("pack");
        if pack_dir.exists() {
            if let Ok(entries) = fs::read_dir(&pack_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|s| s.to_str()) == Some("idx") {
                        if let Ok(Some(obj)) = self.read_from_pack_idx(&path, oid) {
                            return Ok(obj);
                        }
                    }
                }
            }
        }

        // 2. Fallback to git cat-file if available
        self.fallback_git_cat_file(oid)
    }

    fn read_from_pack_idx(&self, idx_path: &Path, oid: &Oid) -> Result<Option<GitObject>, String> {
        let mut idx_file = File::open(idx_path).map_err(|e| e.to_string())?;

        let mut magic = [0u8; 4];
        if idx_file.read_exact(&mut magic).is_err() || &magic != b"\xfftOc" {
            return Ok(None); // Version 1 idx or invalid
        }

        let version = idx_file.read_u32::<BigEndian>().map_err(|e| e.to_string())?;
        if version != 2 {
            return Ok(None);
        }

        // Read fanout table (256 * 4 bytes)
        let mut fanout = [0u32; 256];
        for i in 0..256 {
            fanout[i] = idx_file.read_u32::<BigEndian>().map_err(|e| e.to_string())?;
        }

        let total_objects = fanout[255] as usize;
        let first_byte = oid.as_bytes()[0] as usize;

        let low = if first_byte == 0 { 0 } else { fanout[first_byte - 1] as usize };
        let high = fanout[first_byte] as usize;

        if low >= high {
            return Ok(None);
        }

        // Position of SHA-1 table: 8 (header) + 256 * 4 (fanout) = 1032 bytes
        let sha_table_offset = 1032;

        // Binary search within [low, high)
        let mut left = low;
        let mut right = high;
        let mut found_index = None;

        while left < right {
            let mid = (left + right) / 2;
            let pos = sha_table_offset + mid * 20;
            idx_file.seek(SeekFrom::Start(pos as u64)).map_err(|e| e.to_string())?;

            let mut mid_oid_bytes = [0u8; 20];
            idx_file.read_exact(&mut mid_oid_bytes).map_err(|e| e.to_string())?;
            let mid_oid = Oid::from_bytes(mid_oid_bytes);

            match mid_oid.cmp(oid) {
                std::cmp::Ordering::Less => left = mid + 1,
                std::cmp::Ordering::Greater => right = mid,
                std::cmp::Ordering::Equal => {
                    found_index = Some(mid);
                    break;
                }
            }
        }

        let obj_index = match found_index {
            Some(idx) => idx,
            None => return Ok(None),
        };

        // Offset table starts after SHA-1 table (total_objects * 20) and CRC32 table (total_objects * 4)
        let offset_table_start = 1032 + total_objects * 20 + total_objects * 4;
        let offset_pos = offset_table_start + obj_index * 4;
        idx_file.seek(SeekFrom::Start(offset_pos as u64)).map_err(|e| e.to_string())?;

        let offset_entry = idx_file.read_u32::<BigEndian>().map_err(|e| e.to_string())?;
        let pack_offset = if offset_entry & 0x8000_0000 != 0 {
            // Large offset table (after 4-byte offset table)
            let large_offset_index = (offset_entry & 0x7FFF_FFFF) as usize;
            let large_offset_start = offset_table_start + total_objects * 4;
            let large_pos = large_offset_start + large_offset_index * 8;
            idx_file.seek(SeekFrom::Start(large_pos as u64)).map_err(|e| e.to_string())?;
            idx_file.read_u64::<BigEndian>().map_err(|e| e.to_string())?
        } else {
            offset_entry as u64
        };

        // Read object from .pack file
        let pack_path = idx_path.with_extension("pack");
        self.read_object_at_pack_offset(&pack_path, pack_offset, oid)
    }

    fn read_object_at_pack_offset(
        &self,
        pack_path: &Path,
        offset: u64,
        oid: &Oid,
    ) -> Result<Option<GitObject>, String> {
        let mut pack_file = File::open(pack_path).map_err(|e| e.to_string())?;
        pack_file.seek(SeekFrom::Start(offset)).map_err(|e| e.to_string())?;

        let mut byte = pack_file.read_u8().map_err(|e| e.to_string())?;
        let obj_type_raw = (byte >> 4) & 0x07;
        let mut _uncompressed_size = (byte & 0x0F) as usize;
        let mut shift = 4;

        while byte & 0x80 != 0 {
            byte = pack_file.read_u8().map_err(|e| e.to_string())?;
            _uncompressed_size |= ((byte & 0x7F) as usize) << shift;
            shift += 7;
        }

        match obj_type_raw {
            1 | 2 | 3 | 4 => {
                let obj_type = match obj_type_raw {
                    1 => ObjectType::Commit,
                    2 => ObjectType::Tree,
                    3 => ObjectType::Blob,
                    4 => ObjectType::Tag,
                    _ => unreachable!(),
                };

                let mut decoder = ZlibDecoder::new(pack_file);
                let mut content = Vec::new();
                decoder.read_to_end(&mut content).map_err(|e| e.to_string())?;

                Ok(Some(GitObject { obj_type, content }))
            }
            6 | 7 => {
                // For delta objects (OFS_DELTA or REF_DELTA), use the fallback to ensure exact reconstruction
                self.fallback_git_cat_file(oid).map(Some)
            }
            _ => Ok(None),
        }
    }

    fn fallback_git_cat_file(&self, oid: &Oid) -> Result<GitObject, String> {
        let oid_hex = oid.to_hex();
        let repo_dir = self.git_dir.parent().unwrap_or(&self.git_dir);

        let type_output = Command::new("git")
            .current_dir(repo_dir)
            .args(&["cat-file", "-t", &oid_hex])
            .output();

        let type_str = match type_output {
            Ok(out) if out.status.success() => {
                String::from_utf8_lossy(&out.stdout).trim().to_string()
            }
            _ => return Err(format!("Object {} not found in loose objects or packfiles", oid)),
        };

        let obj_type = ObjectType::from_str(&type_str)?;

        let content_output = Command::new("git")
            .current_dir(repo_dir)
            .args(&["cat-file", &type_str, &oid_hex])
            .output()
            .map_err(|e| format!("Failed to read object {} with git: {}", oid, e))?;

        if !content_output.status.success() {
            return Err(format!("git cat-file failed for object {}", oid));
        }

        Ok(GitObject {
            obj_type,
            content: content_output.stdout,
        })
    }
}
