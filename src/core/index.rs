use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::Read;
use std::path::Path;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use sha1::{Digest, Sha1};
use super::hash::Oid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexEntry {
    pub ctime_sec: u32,
    pub ctime_nsec: u32,
    pub mtime_sec: u32,
    pub mtime_nsec: u32,
    pub dev: u32,
    pub ino: u32,
    pub mode: u32,
    pub uid: u32,
    pub gid: u32,
    pub file_size: u32,
    pub oid: Oid,
    pub flags: u16,
    pub path: String,
}

#[derive(Debug, Clone, Default)]
pub struct Index {
    pub entries: BTreeMap<String, IndexEntry>,
}

impl Index {
    pub fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
        }
    }

    pub fn add(&mut self, entry: IndexEntry) {
        self.entries.insert(entry.path.clone(), entry);
    }

    pub fn remove(&mut self, path: &str) -> Option<IndexEntry> {
        self.entries.remove(path)
    }

    pub fn get(&self, path: &str) -> Option<&IndexEntry> {
        self.entries.get(path)
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    pub fn read_from_file(path: &Path) -> Result<Self, String> {
        if !path.exists() {
            return Ok(Self::new());
        }

        let mut file = File::open(path).map_err(|e| format!("Cannot open index {:?}: {}", path, e))?;
        let mut data = Vec::new();
        file.read_to_end(&mut data).map_err(|e| format!("Cannot read index: {}", e))?;

        if data.len() < 12 + 20 {
            // Header is 12 bytes, footer is 20 bytes SHA1
            return Err("Index file too short".to_string());
        }

        // Verify checksum of data excluding trailing 20 bytes
        let content_len = data.len() - 20;
        let mut hasher = Sha1::new();
        hasher.update(&data[..content_len]);
        let calculated_hash = hasher.finalize();
        if calculated_hash.as_slice() != &data[content_len..] {
            return Err("Index file checksum mismatch".to_string());
        }

        let mut cursor = std::io::Cursor::new(&data[..content_len]);

        let mut sig = [0u8; 4];
        cursor.read_exact(&mut sig).map_err(|e| e.to_string())?;
        if &sig != b"DIRC" {
            return Err(format!("Invalid index signature: {:?}", sig));
        }

        let version = cursor.read_u32::<BigEndian>().map_err(|e| e.to_string())?;
        if version != 2 && version != 3 && version != 4 {
            return Err(format!("Unsupported index version: {}", version));
        }

        let num_entries = cursor.read_u32::<BigEndian>().map_err(|e| e.to_string())? as usize;
        let mut entries = BTreeMap::new();

        for _ in 0..num_entries {
            let ctime_sec = cursor.read_u32::<BigEndian>().map_err(|e| e.to_string())?;
            let ctime_nsec = cursor.read_u32::<BigEndian>().map_err(|e| e.to_string())?;
            let mtime_sec = cursor.read_u32::<BigEndian>().map_err(|e| e.to_string())?;
            let mtime_nsec = cursor.read_u32::<BigEndian>().map_err(|e| e.to_string())?;
            let dev = cursor.read_u32::<BigEndian>().map_err(|e| e.to_string())?;
            let ino = cursor.read_u32::<BigEndian>().map_err(|e| e.to_string())?;
            let mode = cursor.read_u32::<BigEndian>().map_err(|e| e.to_string())?;
            let uid = cursor.read_u32::<BigEndian>().map_err(|e| e.to_string())?;
            let gid = cursor.read_u32::<BigEndian>().map_err(|e| e.to_string())?;
            let file_size = cursor.read_u32::<BigEndian>().map_err(|e| e.to_string())?;

            let mut oid_bytes = [0u8; 20];
            cursor.read_exact(&mut oid_bytes).map_err(|e| e.to_string())?;
            let oid = Oid::from_bytes(oid_bytes);

            let flags = cursor.read_u16::<BigEndian>().map_err(|e| e.to_string())?;
            let _path_len = (flags & 0x0FFF) as usize;

            // Read path until null byte
            let mut path_bytes = Vec::new();
            loop {
                let b = cursor.read_u8().map_err(|e| e.to_string())?;
                if b == 0 {
                    break;
                }
                path_bytes.push(b);
            }

            let path_str = String::from_utf8_lossy(&path_bytes).to_string();

            // Null padding: entry size (from ctime up to null bytes) must be a multiple of 8
            // Fixed fields = 40 (stat) + 20 (oid) + 2 (flags) = 62 bytes
            // + path_bytes.len() + 1 (null byte)
            let entry_len = 62 + path_bytes.len() + 1;
            let pad = (8 - (entry_len % 8)) % 8;
            for _ in 0..pad {
                let _ = cursor.read_u8();
            }

            entries.insert(path_str.clone(), IndexEntry {
                ctime_sec,
                ctime_nsec,
                mtime_sec,
                mtime_nsec,
                dev,
                ino,
                mode,
                uid,
                gid,
                file_size,
                oid,
                flags,
                path: path_str,
            });
        }

        Ok(Self { entries })
    }

    pub fn write_to_file(&self, path: &Path) -> Result<(), String> {
        let mut buffer = Vec::new();

        // 12-byte header: DIRC, version 2, entry count
        buffer.extend_from_slice(b"DIRC");
        buffer.write_u32::<BigEndian>(2).unwrap();
        buffer.write_u32::<BigEndian>(self.entries.len() as u32).unwrap();

        for entry in self.entries.values() {
            buffer.write_u32::<BigEndian>(entry.ctime_sec).unwrap();
            buffer.write_u32::<BigEndian>(entry.ctime_nsec).unwrap();
            buffer.write_u32::<BigEndian>(entry.mtime_sec).unwrap();
            buffer.write_u32::<BigEndian>(entry.mtime_nsec).unwrap();
            buffer.write_u32::<BigEndian>(entry.dev).unwrap();
            buffer.write_u32::<BigEndian>(entry.ino).unwrap();
            buffer.write_u32::<BigEndian>(entry.mode).unwrap();
            buffer.write_u32::<BigEndian>(entry.uid).unwrap();
            buffer.write_u32::<BigEndian>(entry.gid).unwrap();
            buffer.write_u32::<BigEndian>(entry.file_size).unwrap();
            buffer.extend_from_slice(entry.oid.as_bytes());

            let path_bytes = entry.path.as_bytes();
            let len_field = (path_bytes.len().min(0xFFF)) as u16;
            let flags = (entry.flags & 0xF000) | len_field;
            buffer.write_u16::<BigEndian>(flags).unwrap();

            buffer.extend_from_slice(path_bytes);
            buffer.push(0); // terminating null

            // 8-byte alignment padding
            let entry_len = 62 + path_bytes.len() + 1;
            let pad = (8 - (entry_len % 8)) % 8;
            for _ in 0..pad {
                buffer.push(0);
            }
        }

        // Calculate and append SHA-1 checksum
        let mut hasher = Sha1::new();
        hasher.update(&buffer);
        let checksum = hasher.finalize();
        buffer.extend_from_slice(&checksum);

        // Atomic write via temp file
        let tmp_path = path.with_extension(format!("tmp.{}", std::process::id()));
        fs::write(&tmp_path, &buffer).map_err(|e| format!("Failed to write index tmp: {}", e))?;
        fs::rename(&tmp_path, path).map_err(|e| format!("Failed to rename index tmp: {}", e))?;

        Ok(())
    }
}
