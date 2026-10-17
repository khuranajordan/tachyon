use std::fs::{self};
use std::path::{Path, PathBuf};
use super::hash::Oid;
use super::index::Index;
use super::object::ObjectStore;

pub struct Repository {
    pub work_dir: PathBuf,
    pub git_dir: PathBuf,
    pub objects: ObjectStore,
}

impl Repository {
    pub fn find_root() -> Result<Self, String> {
        let current = std::env::current_dir()
            .map_err(|e| format!("Failed to get current directory: {}", e))?;
        Self::find_at(&current)
    }

    pub fn find_at(start_path: &Path) -> Result<Self, String> {
        let mut curr = start_path.to_path_buf();
        loop {
            let git_dir = curr.join(".git");
            if git_dir.exists() {
                if git_dir.is_dir() {
                    return Ok(Self {
                        work_dir: curr.clone(),
                        git_dir: git_dir.clone(),
                        objects: ObjectStore::new(&git_dir),
                    });
                } else if git_dir.is_file() {
                    // Submodule or gitdir file
                    if let Ok(content) = fs::read_to_string(&git_dir) {
                        if let Some(path_str) = content.trim().strip_prefix("gitdir: ") {
                            let resolved = curr.join(path_str);
                            return Ok(Self {
                                work_dir: curr.clone(),
                                git_dir: resolved.clone(),
                                objects: ObjectStore::new(&resolved),
                            });
                        }
                    }
                }
            }

            if !curr.pop() {
                break;
            }
        }

        Err("fatal: not a git repository (or any of the parent directories): .git".to_string())
    }

    pub fn init(work_dir: &Path, initial_branch: &str) -> Result<Self, String> {
        let git_dir = work_dir.join(".git");
        if git_dir.exists() {
            return Err("Repository already exists".to_string());
        }

        fs::create_dir_all(git_dir.join("objects/info"))
            .map_err(|e| format!("Failed to create objects/info: {}", e))?;
        fs::create_dir_all(git_dir.join("objects/pack"))
            .map_err(|e| format!("Failed to create objects/pack: {}", e))?;
        fs::create_dir_all(git_dir.join("refs/heads"))
            .map_err(|e| format!("Failed to create refs/heads: {}", e))?;
        fs::create_dir_all(git_dir.join("refs/tags"))
            .map_err(|e| format!("Failed to create refs/tags: {}", e))?;

        // Write HEAD
        let head_content = format!("ref: refs/heads/{}\n", initial_branch);
        fs::write(git_dir.join("HEAD"), head_content)
            .map_err(|e| format!("Failed to write HEAD: {}", e))?;

        // Write initial config
        let config_content = "[core]\n\trepositoryformatversion = 0\n\tfilemode = true\n\tbare = false\n\tlogallrefupdates = true\n\tignorecase = true\n\tprecomposeunicode = true\n";
        fs::write(git_dir.join("config"), config_content)
            .map_err(|e| format!("Failed to write config: {}", e))?;

        // Write description
        let desc = "Unnamed repository; edit this file 'description' to name the repository.\n";
        fs::write(git_dir.join("description"), desc)
            .map_err(|e| format!("Failed to write description: {}", e))?;

        Ok(Self {
            work_dir: work_dir.to_path_buf(),
            git_dir: git_dir.clone(),
            objects: ObjectStore::new(&git_dir),
        })
    }

    pub fn read_head(&self) -> Result<HeadState, String> {
        let head_path = self.git_dir.join("HEAD");
        if !head_path.exists() {
            return Err("HEAD file does not exist".to_string());
        }

        let content = fs::read_to_string(&head_path)
            .map_err(|e| format!("Failed to read HEAD: {}", e))?;
        let trimmed = content.trim();

        if let Some(branch_ref) = trimmed.strip_prefix("ref: refs/heads/") {
            let ref_path = self.git_dir.join("refs/heads").join(branch_ref);
            let commit_oid = if ref_path.exists() {
                let ref_content = fs::read_to_string(&ref_path)
                    .map_err(|e| format!("Failed to read ref {:?}: {}", ref_path, e))?;
                Some(Oid::from_hex(ref_content.trim())?)
            } else {
                None
            };
            Ok(HeadState::Branch {
                name: branch_ref.to_string(),
                commit: commit_oid,
            })
        } else {
            let oid = Oid::from_hex(trimmed)?;
            Ok(HeadState::Detached(oid))
        }
    }

    pub fn current_branch_name(&self) -> String {
        match self.read_head() {
            Ok(HeadState::Branch { name, .. }) => name,
            Ok(HeadState::Detached(oid)) => format!("detached@{}", oid.short()),
            Err(_) => "HEAD".to_string(),
        }
    }

    pub fn head_commit(&self) -> Result<Option<Oid>, String> {
        match self.read_head()? {
            HeadState::Branch { commit, .. } => Ok(commit),
            HeadState::Detached(oid) => Ok(Some(oid)),
        }
    }

    pub fn set_head_branch(&self, branch_name: &str) -> Result<(), String> {
        let head_path = self.git_dir.join("HEAD");
        let content = format!("ref: refs/heads/{}\n", branch_name);
        fs::write(head_path, content)
            .map_err(|e| format!("Failed to update HEAD: {}", e))
    }

    pub fn set_head_detached(&self, oid: &Oid) -> Result<(), String> {
        let head_path = self.git_dir.join("HEAD");
        let content = format!("{}\n", oid.to_hex());
        fs::write(head_path, content)
            .map_err(|e| format!("Failed to update HEAD: {}", e))
    }

    pub fn read_branch(&self, branch_name: &str) -> Result<Oid, String> {
        let ref_path = self.git_dir.join("refs/heads").join(branch_name);
        if !ref_path.exists() {
            return Err(format!("Branch '{}' does not exist", branch_name));
        }
        let content = fs::read_to_string(&ref_path)
            .map_err(|e| format!("Failed to read branch {}: {}", branch_name, e))?;
        Oid::from_hex(content.trim())
    }

    pub fn write_branch(&self, branch_name: &str, oid: &Oid) -> Result<(), String> {
        let ref_path = self.git_dir.join("refs/heads").join(branch_name);
        if let Some(parent) = ref_path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        fs::write(&ref_path, format!("{}\n", oid.to_hex()))
            .map_err(|e| format!("Failed to write branch ref {}: {}", branch_name, e))
    }

    pub fn list_branches(&self) -> Result<Vec<(String, Oid, bool)>, String> {
        let heads_dir = self.git_dir.join("refs/heads");
        let mut branches = Vec::new();
        let current_branch = self.current_branch_name();

        if heads_dir.exists() {
            for entry in walkdir::WalkDir::new(&heads_dir).into_iter().filter_map(|e| e.ok()) {
                if entry.file_type().is_file() {
                    let rel = entry.path().strip_prefix(&heads_dir)
                        .map_err(|e| e.to_string())?;
                    let name = rel.to_string_lossy().to_string();
                    if let Ok(content) = fs::read_to_string(entry.path()) {
                        if let Ok(oid) = Oid::from_hex(content.trim()) {
                            let is_current = name == current_branch;
                            branches.push((name, oid, is_current));
                        }
                    }
                }
            }
        }

        branches.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(branches)
    }

    pub fn create_branch(&self, name: &str, target: &Oid) -> Result<(), String> {
        let ref_path = self.git_dir.join("refs/heads").join(name);
        if ref_path.exists() {
            return Err(format!("A branch named '{}' already exists.", name));
        }
        self.write_branch(name, target)
    }

    pub fn delete_branch(&self, name: &str) -> Result<(), String> {
        let current = self.current_branch_name();
        if current == name {
            return Err(format!("Cannot delete current branch '{}'", name));
        }
        let ref_path = self.git_dir.join("refs/heads").join(name);
        if !ref_path.exists() {
            return Err(format!("Branch '{}' not found", name));
        }
        fs::remove_file(ref_path).map_err(|e| format!("Failed to delete branch {}: {}", name, e))
    }

    pub fn list_tags(&self) -> Result<Vec<(String, Oid)>, String> {
        let tags_dir = self.git_dir.join("refs/tags");
        let mut tags = Vec::new();
        if tags_dir.exists() {
            for entry in fs::read_dir(tags_dir).map_err(|e| e.to_string())?.flatten() {
                if let Ok(ft) = entry.file_type() {
                    if ft.is_file() {
                        let name = entry.file_name().to_string_lossy().to_string();
                        if let Ok(content) = fs::read_to_string(entry.path()) {
                            if let Ok(oid) = Oid::from_hex(content.trim()) {
                                tags.push((name, oid));
                            }
                        }
                    }
                }
            }
        }
        tags.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(tags)
    }

    pub fn create_tag(&self, name: &str, target: &Oid) -> Result<(), String> {
        let tag_path = self.git_dir.join("refs/tags").join(name);
        if tag_path.exists() {
            return Err(format!("Tag '{}' already exists", name));
        }
        fs::write(tag_path, format!("{}\n", target.to_hex()))
            .map_err(|e| format!("Failed to write tag {}: {}", name, e))
    }

    pub fn resolve_oid(&self, prefix: &str) -> Result<Oid, String> {
        let trimmed = prefix.trim();
        if trimmed.len() == 40 {
            return Oid::from_hex(trimmed);
        }
        if trimmed.len() < 4 {
            return Err("Commit prefix too short (minimum 4 characters)".to_string());
        }

        let (dir_prefix, file_prefix) = trimmed.split_at(2);
        let obj_dir = self.git_dir.join("objects").join(dir_prefix);
        if obj_dir.exists() {
            let mut matches = Vec::new();
            if let Ok(entries) = fs::read_dir(&obj_dir) {
                for entry in entries.flatten() {
                    let file_name = entry.file_name().to_string_lossy().to_string();
                    if file_name.starts_with(file_prefix) {
                        let full_hex = format!("{}{}", dir_prefix, file_name);
                        if let Ok(oid) = Oid::from_hex(&full_hex) {
                            matches.push(oid);
                        }
                    }
                }
            }
            if matches.len() == 1 {
                return Ok(matches[0]);
            } else if matches.len() > 1 {
                return Err(format!("Short hash '{}' is ambiguous (matches {} objects)", prefix, matches.len()));
            }
        }

        Err(format!("Could not resolve object '{}'", prefix))
    }

    pub fn read_index(&self) -> Result<Index, String> {
        Index::read_from_file(&self.git_dir.join("index"))
    }

    pub fn write_index(&self, index: &Index) -> Result<(), String> {
        index.write_to_file(&self.git_dir.join("index"))
    }

    pub fn author_identity(&self) -> (String, String) {
        // Try git config, then env vars, then fallback
        let mut name = std::env::var("GIT_AUTHOR_NAME").ok();
        let mut email = std::env::var("GIT_AUTHOR_EMAIL").ok();

        if name.is_none() || email.is_none() {
            // Check repo config or ~/.gitconfig
            let configs = [
                self.git_dir.join("config"),
                dirs_fallback_gitconfig(),
            ];

            for cfg in &configs {
                if cfg.exists() {
                    if let Ok(text) = fs::read_to_string(cfg) {
                        for line in text.lines() {
                            let trimmed = line.trim();
                            if name.is_none() && trimmed.starts_with("name =") {
                                name = Some(trimmed["name =".len()..].trim().to_string());
                            }
                            if email.is_none() && trimmed.starts_with("email =") {
                                email = Some(trimmed["email =".len()..].trim().to_string());
                            }
                        }
                    }
                }
            }
        }

        (
            name.unwrap_or_else(|| "Tachyon User".to_string()),
            email.unwrap_or_else(|| "tachyon@speed.vcs".to_string()),
        )
    }
}

fn dirs_fallback_gitconfig() -> PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".gitconfig")
    } else {
        PathBuf::from("/tmp/.gitconfig")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HeadState {
    Branch { name: String, commit: Option<Oid> },
    Detached(Oid),
}
