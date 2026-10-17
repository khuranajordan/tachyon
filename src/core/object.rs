use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use super::hash::Oid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObjectType {
    Blob,
    Tree,
    Commit,
    Tag,
}

impl ObjectType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ObjectType::Blob => "blob",
            ObjectType::Tree => "tree",
            ObjectType::Commit => "commit",
            ObjectType::Tag => "tag",
        }
    }

    pub fn from_str(s: &str) -> Result<Self, String> {
        match s {
            "blob" => Ok(ObjectType::Blob),
            "tree" => Ok(ObjectType::Tree),
            "commit" => Ok(ObjectType::Commit),
            "tag" => Ok(ObjectType::Tag),
            other => Err(format!("Unknown object type: {}", other)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitObject {
    pub obj_type: ObjectType,
    pub content: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeEntry {
    pub mode: u32,
    pub name: String,
    pub oid: Oid,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitObject {
    pub tree: Oid,
    pub parents: Vec<Oid>,
    pub author: String,
    pub committer: String,
    pub message: String,
}

impl GitObject {
    pub fn blob(data: Vec<u8>) -> Self {
        Self {
            obj_type: ObjectType::Blob,
            content: data,
        }
    }

    pub fn oid(&self) -> Oid {
        Oid::hash_object(self.obj_type.as_str(), &self.content)
    }

    pub fn serialize_with_header(&self) -> Vec<u8> {
        let header = format!("{} {}\0", self.obj_type.as_str(), self.content.len());
        let mut bytes = Vec::with_capacity(header.len() + self.content.len());
        bytes.extend_from_slice(header.as_bytes());
        bytes.extend_from_slice(&self.content);
        bytes
    }
}

pub struct ObjectStore {
    git_dir: PathBuf,
    objects_dir: PathBuf,
}

impl ObjectStore {
    pub fn new(git_dir: &Path) -> Self {
        Self {
            git_dir: git_dir.to_path_buf(),
            objects_dir: git_dir.join("objects"),
        }
    }

    pub fn object_path(&self, oid: &Oid) -> PathBuf {
        let hex = oid.to_hex();
        let (dir, file) = hex.split_at(2);
        self.objects_dir.join(dir).join(file)
    }

    pub fn exists(&self, oid: &Oid) -> bool {
        if self.object_path(oid).exists() {
            return true;
        }
        let pack_mgr = super::pack::PackManager::new(&self.git_dir);
        pack_mgr.read_object(oid).is_ok()
    }

    pub fn read(&self, oid: &Oid) -> Result<GitObject, String> {
        let path = self.object_path(oid);
        if !path.exists() {
            let pack_mgr = super::pack::PackManager::new(&self.git_dir);
            return pack_mgr.read_object(oid);
        }

        let file = File::open(&path).map_err(|e| format!("Failed to open {:?}: {}", path, e))?;
        let mut decoder = ZlibDecoder::new(file);
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed)
            .map_err(|e| format!("Failed to decompress object {}: {}", oid, e))?;

        // Format is: `<type> <size>\0<content>`
        let null_pos = decompressed.iter().position(|&b| b == 0)
            .ok_or_else(|| format!("Corrupt object {}: missing null byte in header", oid))?;

        let header_str = std::str::from_utf8(&decompressed[..null_pos])
            .map_err(|e| format!("Invalid UTF-8 header in {}: {}", oid, e))?;

        let mut parts = header_str.split(' ');
        let type_str = parts.next().ok_or_else(|| "Missing type in object header")?;
        let _size_str = parts.next().ok_or_else(|| "Missing size in object header")?;

        let obj_type = ObjectType::from_str(type_str)?;
        let content = decompressed[null_pos + 1..].to_vec();

        Ok(GitObject { obj_type, content })
    }

    pub fn write(&self, obj: &GitObject) -> Result<Oid, String> {
        let oid = obj.oid();
        let path = self.object_path(&oid);
        if path.exists() {
            return Ok(oid);
        }

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create object directory {:?}: {}", parent, e))?;
        }

        let serialized = obj.serialize_with_header();
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::fast());
        encoder.write_all(&serialized)
            .map_err(|e| format!("Compression failed: {}", e))?;
        let compressed = encoder.finish()
            .map_err(|e| format!("Compression finish failed: {}", e))?;

        // Write to temp file then rename for atomic write
        let tmp_path = path.with_extension(format!("tmp.{}", std::process::id()));
        fs::write(&tmp_path, &compressed)
            .map_err(|e| format!("Failed to write tmp object {:?}: {}", tmp_path, e))?;

        fs::rename(&tmp_path, &path)
            .map_err(|e| format!("Failed to rename {:?} to {:?}: {}", tmp_path, path, e))?;

        Ok(oid)
    }

    pub fn write_blob(&self, data: &[u8]) -> Result<Oid, String> {
        let obj = GitObject::blob(data.to_vec());
        self.write(&obj)
    }

    pub fn parse_tree(&self, oid: &Oid) -> Result<Vec<TreeEntry>, String> {
        let obj = self.read(oid)?;
        if obj.obj_type != ObjectType::Tree {
            return Err(format!("Object {} is {:?}, not a tree", oid, obj.obj_type));
        }

        let mut entries = Vec::new();
        let bytes = &obj.content;
        let mut idx = 0;

        while idx < bytes.len() {
            // Find space after mode
            let space_idx = bytes[idx..].iter().position(|&b| b == b' ')
                .ok_or_else(|| "Malformed tree entry: missing space after mode")? + idx;
            let mode_str = std::str::from_utf8(&bytes[idx..space_idx])
                .map_err(|e| format!("Invalid mode in tree: {}", e))?;
            let mode = u32::from_str_radix(mode_str, 8)
                .map_err(|e| format!("Invalid octal mode '{}': {}", mode_str, e))?;

            // Find null byte after filename
            let null_idx = bytes[space_idx + 1..].iter().position(|&b| b == 0)
                .ok_or_else(|| "Malformed tree entry: missing null after name")? + (space_idx + 1);
            let name = std::str::from_utf8(&bytes[space_idx + 1..null_idx])
                .map_err(|e| format!("Invalid name in tree: {}", e))?
                .to_string();

            // Next 20 bytes is Oid
            let oid_start = null_idx + 1;
            let oid_end = oid_start + 20;
            if oid_end > bytes.len() {
                return Err("Malformed tree entry: unexpected EOF for OID".to_string());
            }

            let mut oid_bytes = [0u8; 20];
            oid_bytes.copy_from_slice(&bytes[oid_start..oid_end]);
            let entry_oid = Oid::from_bytes(oid_bytes);

            entries.push(TreeEntry {
                mode,
                name,
                oid: entry_oid,
            });

            idx = oid_end;
        }

        Ok(entries)
    }

    pub fn serialize_tree(entries: &[TreeEntry]) -> Vec<u8> {
        let mut sorted = entries.to_vec();
        // Git sorts trees specially: directories are treated as if having trailing '/'
        sorted.sort_by(|a, b| {
            let a_name = if a.mode == 0o040000 { format!("{}/", a.name) } else { a.name.clone() };
            let b_name = if b.mode == 0o040000 { format!("{}/", b.name) } else { b.name.clone() };
            a_name.cmp(&b_name)
        });

        let mut content = Vec::new();
        for entry in sorted {
            let mode_str = format!("{:o}", entry.mode);
            content.extend_from_slice(mode_str.as_bytes());
            content.push(b' ');
            content.extend_from_slice(entry.name.as_bytes());
            content.push(0);
            content.extend_from_slice(entry.oid.as_bytes());
        }
        content
    }

    pub fn write_tree(&self, entries: &[TreeEntry]) -> Result<Oid, String> {
        let content = Self::serialize_tree(entries);
        let obj = GitObject {
            obj_type: ObjectType::Tree,
            content,
        };
        self.write(&obj)
    }

    pub fn parse_commit(&self, oid: &Oid) -> Result<CommitObject, String> {
        let obj = self.read(oid)?;
        if obj.obj_type != ObjectType::Commit {
            return Err(format!("Object {} is not a commit", oid));
        }

        let text = std::str::from_utf8(&obj.content)
            .map_err(|e| format!("Invalid UTF-8 in commit {}: {}", oid, e))?;

        let mut tree: Option<Oid> = None;
        let mut parents = Vec::new();
        let mut author = String::new();
        let mut committer = String::new();
        let mut in_message = false;
        let mut message_lines = Vec::new();

        for line in text.lines() {
            if in_message {
                message_lines.push(line);
                continue;
            }

            if line.is_empty() {
                in_message = true;
                continue;
            }

            if let Some(t) = line.strip_prefix("tree ") {
                tree = Some(Oid::from_hex(t)?);
            } else if let Some(p) = line.strip_prefix("parent ") {
                parents.push(Oid::from_hex(p)?);
            } else if let Some(a) = line.strip_prefix("author ") {
                author = a.to_string();
            } else if let Some(c) = line.strip_prefix("committer ") {
                committer = c.to_string();
            }
        }

        let tree = tree.ok_or_else(|| format!("Commit {} missing tree field", oid))?;
        let message = message_lines.join("\n").trim().to_string();

        Ok(CommitObject {
            tree,
            parents,
            author,
            committer,
            message,
        })
    }

    pub fn write_commit(&self, commit: &CommitObject) -> Result<Oid, String> {
        let mut content = format!("tree {}\n", commit.tree.to_hex());
        for parent in &commit.parents {
            content.push_str(&format!("parent {}\n", parent.to_hex()));
        }
        content.push_str(&format!("author {}\n", commit.author));
        content.push_str(&format!("committer {}\n\n", commit.committer));
        content.push_str(&commit.message);
        content.push('\n');

        let obj = GitObject {
            obj_type: ObjectType::Commit,
            content: content.into_bytes(),
        };
        self.write(&obj)
    }
}
