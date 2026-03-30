use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GutterMark {
    Added,
    Modified,
    Deleted,
}

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

    pub fn diff_for_file(project_root: &Path, file_path: &Path) -> HashMap<usize, GutterMark> {
        let mut marks = HashMap::new();
        let rel = file_path.strip_prefix(project_root).unwrap_or(file_path);

        let output = Command::new("git")
            .args(["diff", "--unified=0", "--no-color", "--"])
            .arg(rel)
            .current_dir(project_root)
            .output();

        let output = match output {
            Ok(o) if o.status.success() => o,
            _ => return marks,
        };

        let text = String::from_utf8_lossy(&output.stdout);
        for line in text.lines() {
            if !line.starts_with("@@") {
                continue;
            }
            // parse @@ -old_start,old_count +new_start,new_count @@
            let parts: Vec<&str> = line.split(' ').collect();
            if parts.len() < 3 {
                continue;
            }
            let old_part = parts[1]; // -start,count
            let new_part = parts[2]; // +start,count

            let old_count: usize = old_part
                .split(',')
                .nth(1)
                .unwrap_or("1")
                .parse()
                .unwrap_or(1);
            let new_start: usize = new_part
                .trim_start_matches('+')
                .split(',')
                .next()
                .unwrap_or("1")
                .parse()
                .unwrap_or(1);
            let new_count: usize = new_part
                .split(',')
                .nth(1)
                .unwrap_or("1")
                .parse()
                .unwrap_or(1);

            if old_count == 0 && new_count > 0 {
                // pure addition
                for i in 0..new_count {
                    marks.insert(new_start - 1 + i, GutterMark::Added);
                }
            } else if new_count == 0 && old_count > 0 {
                // pure deletion — mark the line after where deletion happened
                if new_start > 0 {
                    marks.insert(new_start - 1, GutterMark::Deleted);
                }
            } else {
                // modification
                for i in 0..new_count {
                    marks.insert(new_start - 1 + i, GutterMark::Modified);
                }
            }
        }

        marks
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
