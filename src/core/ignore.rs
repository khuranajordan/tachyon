use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct GitIgnore {
    patterns: Vec<IgnorePattern>,
}

#[derive(Debug, Clone)]
struct IgnorePattern {
    pattern: String,
    negative: bool,
    dir_only: bool,
}

impl GitIgnore {
    pub fn new(repo_root: &Path) -> Self {
        let mut patterns = Vec::new();

        // Default ignores
        patterns.push(IgnorePattern {
            pattern: ".git".to_string(),
            negative: false,
            dir_only: false,
        });

        // Load .gitignore if present in repo root
        let gitignore_path = repo_root.join(".gitignore");
        if gitignore_path.exists() {
            if let Ok(content) = fs::read_to_string(&gitignore_path) {
                for line in content.lines() {
                    let trimmed = line.trim();
                    if trimmed.is_empty() || trimmed.starts_with('#') {
                        continue;
                    }

                    let mut pat = trimmed;
                    let negative = if pat.starts_with('!') {
                        pat = &pat[1..];
                        true
                    } else {
                        false
                    };

                    let dir_only = if pat.ends_with('/') {
                        pat = &pat[..pat.len() - 1];
                        true
                    } else {
                        false
                    };

                    let pat = pat.trim_start_matches('/');

                    patterns.push(IgnorePattern {
                        pattern: pat.to_string(),
                        negative,
                        dir_only,
                    });
                }
            }
        }

        Self { patterns }
    }

    pub fn is_ignored(&self, relative_path: &str, is_dir: bool) -> bool {
        let norm_path = relative_path.replace('\\', "/");
        let components: Vec<&str> = norm_path.split('/').filter(|s| !s.is_empty()).collect();

        if components.contains(&".git") {
            return true;
        }

        let mut ignored = false;

        for pat in &self.patterns {
            if pat.dir_only && !is_dir {
                continue;
            }

            if self.matches_pattern(&norm_path, &components, &pat.pattern) {
                ignored = !pat.negative;
            }
        }

        ignored
    }

    fn matches_pattern(&self, full_path: &str, components: &[&str], pattern: &str) -> bool {
        // Direct match
        if full_path == pattern {
            return true;
        }

        // Match against any path segment (e.g., target, *.o)
        for comp in components {
            if *comp == pattern {
                return true;
            }
            if pattern.starts_with('*') && pattern.len() > 1 {
                let suffix = &pattern[1..];
                if comp.ends_with(suffix) {
                    return true;
                }
            }
        }

        // Path prefix match (e.g. dir/* or dir/file)
        if full_path.starts_with(&format!("{}/", pattern)) {
            return true;
        }

        false
    }
}
