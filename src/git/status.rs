use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Default)]
pub struct GitInfo {
    pub branch: String,
    pub file_statuses: HashMap<PathBuf, char>,
    pub changed: usize,
    pub staged: usize,
    pub ahead: usize,
    pub behind: usize,
}

impl GitInfo {
    pub fn gather(project_root: &Path) -> Option<Self> {
        let mut info = GitInfo::default();

        // get branch name
        let branch_output = Command::new("git")
            .args(["rev-parse", "--abbrev-ref", "HEAD"])
            .current_dir(project_root)
            .output()
            .ok()?;

        if !branch_output.status.success() {
            return None; // not a git repo
        }
        info.branch = String::from_utf8_lossy(&branch_output.stdout)
            .trim()
            .to_string();

        // get file statuses
        let status_output = Command::new("git")
            .args(["status", "--porcelain"])
            .current_dir(project_root)
            .output()
            .ok()?;

        for line in String::from_utf8_lossy(&status_output.stdout).lines() {
            if line.len() < 4 {
                continue;
            }
            let x = line.chars().nth(0).unwrap_or(' ');
            let y = line.chars().nth(1).unwrap_or(' ');
            let file_path = PathBuf::from(line[3..].trim());

            let status_char = if x == '?' && y == '?' {
                '?'
            } else if x == 'A' || y == 'A' {
                'A'
            } else if x == 'D' || y == 'D' {
                'D'
            } else if x == 'M' || y == 'M' {
                'M'
            } else if x == 'R' || y == 'R' {
                'R'
            } else if x == 'U' || y == 'U' {
                'U'
            } else {
                'M'
            };

            if x != ' ' && x != '?' {
                info.staged += 1;
            }
            if y != ' ' && y != '?' {
                info.changed += 1;
            }
            if x == '?' {
                info.changed += 1;
            }

            info.file_statuses.insert(file_path, status_char);
        }

        // get ahead/behind
        let ab_output = Command::new("git")
            .args(["rev-list", "--left-right", "--count", "HEAD...@{upstream}"])
            .current_dir(project_root)
            .output()
            .ok();

        if let Some(output) = ab_output {
            if output.status.success() {
                let text = String::from_utf8_lossy(&output.stdout);
                let parts: Vec<&str> = text.trim().split('\t').collect();
                if parts.len() == 2 {
                    info.ahead = parts[0].parse().unwrap_or(0);
                    info.behind = parts[1].parse().unwrap_or(0);
                }
            }
        }

        Some(info)
    }

    pub fn status_for_file(&self, rel_path: &Path) -> Option<char> {
        self.file_statuses.get(rel_path).copied()
    }

    pub fn status_line(&self) -> String {
        let mut parts = vec![self.branch.clone()];
        if self.changed > 0 {
            parts.push(format!("~{}", self.changed));
        }
        if self.staged > 0 {
            parts.push(format!("+{}", self.staged));
        }
        if self.ahead > 0 {
            parts.push(format!("↑{}", self.ahead));
        }
        if self.behind > 0 {
            parts.push(format!("↓{}", self.behind));
        }
        parts.join(" ")
    }
}
