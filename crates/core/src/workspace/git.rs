use anyhow::Result;
use std::path::Path;
use tokio::process::Command;

use protocol::{BranchInfo, ChangedFile, CommitInfo, GitState, RemoteBranchInfo};

pub async fn refresh_git(repo: &Path) -> Result<GitState> {
    let branch_fut = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(repo)
        .output();

    let status_fut = Command::new("git")
        .args(["status", "--porcelain=v1"])
        .current_dir(repo)
        .output();

    let upstream_fut = get_upstream_status(repo);
    let commits_fut = get_recent_commits(repo, 20);
    let local_branches_fut = get_local_branches(repo);
    let remote_branches_fut = get_remote_branches(repo);

    let (branch_out, status_out, (upstream, ahead, behind), recent_commits, local_branches, remote_branches) =
        tokio::join!(branch_fut, status_fut, upstream_fut, commits_fut, local_branches_fut, remote_branches_fut);

    let branch = match branch_out {
        Ok(out) if out.status.success() => {
            Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
                .filter(|s| !s.is_empty())
        }
        _ => None,
    };

    let mut changed = Vec::new();
    if let Ok(out) = status_out {
        if out.status.success() {
            for line in String::from_utf8_lossy(&out.stdout).lines() {
                if let Some(file) = parse_porcelain_line(line) {
                    changed.push(file);
                }
            }
        }
    }

    Ok(GitState {
        branch,
        upstream,
        ahead,
        behind,
        changed,
        recent_commits,
        local_branches,
        remote_branches,
    })
}

async fn get_upstream_status(repo: &Path) -> (Option<String>, Option<u32>, Option<u32>) {
    let upstream_out = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{upstream}"])
        .current_dir(repo)
        .output()
        .await;

    let upstream = match upstream_out {
        Ok(out) if out.status.success() => {
            let name = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if name.is_empty() {
                return (None, None, None);
            }
            Some(name)
        }
        _ => return (None, None, None),
    };

    let count_out = Command::new("git")
        .args(["rev-list", "--left-right", "--count", "HEAD...@{upstream}"])
        .current_dir(repo)
        .output()
        .await;

    let (ahead, behind) = match count_out {
        Ok(out) if out.status.success() => {
            let text = String::from_utf8_lossy(&out.stdout);
            let parts: Vec<&str> = text.trim().split('\t').collect();
            if parts.len() == 2 {
                let a = parts[0].parse::<u32>().unwrap_or(0);
                let b = parts[1].parse::<u32>().unwrap_or(0);
                (Some(a), Some(b))
            } else {
                (Some(0), Some(0))
            }
        }
        _ => (None, None),
    };

    (upstream, ahead, behind)
}

pub async fn diff_file(repo: &Path, file: &str) -> Result<String> {
    let out = Command::new("git")
        .arg("diff")
        .arg("--")
        .arg(file)
        .current_dir(repo)
        .output()
        .await?;

    let text = String::from_utf8_lossy(&out.stdout).to_string();
    if !text.trim().is_empty() {
        return Ok(text);
    }

    let tracked = Command::new("git")
        .arg("ls-files")
        .arg("--error-unmatch")
        .arg("--")
        .arg(file)
        .current_dir(repo)
        .output()
        .await
        .map(|o| o.status.success())
        .unwrap_or(false);
    if tracked {
        return Ok(text);
    }

    let full_path = repo.join(file);
    if !full_path.exists() {
        return Ok(text);
    }
    if full_path.is_dir() {
        return Ok(format!(
            "Untracked directory: {file}\n(no file-level diff available)\n"
        ));
    }

    let bytes = std::fs::read(&full_path)?;
    if bytes.iter().any(|b| *b == 0) {
        return Ok(format!("Binary file added: {file}\n"));
    }

    let mut diff = String::new();
    diff.push_str(&format!("diff --git a/{file} b/{file}\n"));
    diff.push_str("new file mode 100644\n");
    diff.push_str("--- /dev/null\n");
    diff.push_str(&format!("+++ b/{file}\n"));
    diff.push_str("@@ -0,0 +1 @@\n");
    for line in String::from_utf8_lossy(&bytes).lines() {
        diff.push('+');
        diff.push_str(line);
        diff.push('\n');
    }
    Ok(diff)
}

fn parse_porcelain_line(line: &str) -> Option<ChangedFile> {
    if line.len() < 3 {
        return None;
    }

    let bytes = line.as_bytes();
    let index_status = bytes[0] as char;
    let worktree_status = bytes[1] as char;
    let path = line[3..].trim().to_string();
    if path.is_empty() {
        return None;
    }

    Some(ChangedFile {
        path,
        index_status,
        worktree_status,
    })
}

async fn get_recent_commits(repo: &Path, count: usize) -> Vec<CommitInfo> {
    let format = "%h\x1f%s\x1f%an\x1f%cr";
    let out = Command::new("git")
        .args(["log", &format!("-{count}"), &format!("--format={format}")])
        .current_dir(repo)
        .output()
        .await;

    let Ok(out) = out else { return Vec::new() };
    if !out.status.success() {
        return Vec::new();
    }

    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.splitn(4, '\x1f').collect();
            if parts.len() == 4 {
                Some(CommitInfo {
                    hash: parts[0].to_string(),
                    message: parts[1].to_string(),
                    author: parts[2].to_string(),
                    date: parts[3].to_string(),
                })
            } else {
                None
            }
        })
        .collect()
}

pub async fn diff_commit(repo: &Path, hash: &str) -> Result<String> {
    let out = Command::new("git")
        .args(["show", hash, "--format="])
        .current_dir(repo)
        .output()
        .await?;
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

pub async fn stage_file(repo: &Path, file: &str) -> Result<()> {
    let out = Command::new("git")
        .args(["add", "--", file])
        .current_dir(repo)
        .output()
        .await?;
    if !out.status.success() {
        anyhow::bail!(
            "git add failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    Ok(())
}

pub async fn unstage_file(repo: &Path, file: &str) -> Result<()> {
    let out = Command::new("git")
        .args(["reset", "HEAD", "--", file])
        .current_dir(repo)
        .output()
        .await?;
    if !out.status.success() {
        anyhow::bail!(
            "git reset failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    Ok(())
}

pub async fn stage_all(repo: &Path) -> Result<()> {
    let out = Command::new("git")
        .args(["add", "-A"])
        .current_dir(repo)
        .output()
        .await?;
    if !out.status.success() {
        anyhow::bail!(
            "git add -A failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    Ok(())
}

pub async fn unstage_all(repo: &Path) -> Result<()> {
    let out = Command::new("git")
        .args(["reset", "HEAD"])
        .current_dir(repo)
        .output()
        .await?;
    if !out.status.success() {
        anyhow::bail!(
            "git reset failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    Ok(())
}

async fn get_local_branches(repo: &Path) -> Vec<BranchInfo> {
    let out = Command::new("git")
        .args(["for-each-ref", "--format=%(HEAD) %(refname:short) %(upstream:track)", "refs/heads/"])
        .current_dir(repo)
        .output()
        .await;

    let Ok(out) = out else { return Vec::new() };
    if !out.status.success() {
        return Vec::new();
    }

    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| {
            let line = line.trim_end();
            if line.trim().is_empty() {
                return None;
            }
            let is_head = line.starts_with('*');
            let rest = &line[2..];
            let (name, track) = if let Some(bracket_start) = rest.find('[') {
                let name = rest[..bracket_start].trim().to_string();
                let track_str = &rest[bracket_start..];
                let (ahead, behind) = parse_track_info(track_str);
                (name, (ahead, behind))
            } else {
                (rest.trim().to_string(), (None, None))
            };
            if name.is_empty() {
                return None;
            }
            Some(BranchInfo {
                name,
                is_head,
                ahead: track.0,
                behind: track.1,
            })
        })
        .collect()
}

fn parse_track_info(info: &str) -> (Option<u32>, Option<u32>) {
    // Parses "[ahead N]", "[behind N]", "[ahead N, behind M]", or "[gone]"
    let trimmed = info.trim().trim_start_matches('[').trim_end_matches(']');
    if trimmed == "gone" || trimmed.is_empty() {
        return (None, None);
    }
    let mut ahead = None;
    let mut behind = None;
    for part in trimmed.split(',') {
        let part = part.trim();
        if let Some(n) = part.strip_prefix("ahead ") {
            ahead = n.trim().parse::<u32>().ok();
        } else if let Some(n) = part.strip_prefix("behind ") {
            behind = n.trim().parse::<u32>().ok();
        }
    }
    (ahead, behind)
}

async fn get_remote_branches(repo: &Path) -> Vec<RemoteBranchInfo> {
    let out = Command::new("git")
        .args(["for-each-ref", "--format=%(refname:short)", "refs/remotes/"])
        .current_dir(repo)
        .output()
        .await;

    let Ok(out) = out else { return Vec::new() };
    if !out.status.success() {
        return Vec::new();
    }

    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|line| !line.trim().is_empty() && !line.trim().ends_with("/HEAD"))
        .map(|line| RemoteBranchInfo {
            full_name: line.trim().to_string(),
        })
        .collect()
}

pub async fn create_branch(repo: &Path, branch: &str) -> Result<()> {
    let out = Command::new("git")
        .args(["checkout", "-b", branch])
        .current_dir(repo)
        .output()
        .await?;
    if !out.status.success() {
        anyhow::bail!(
            "git checkout -b failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    Ok(())
}

pub async fn checkout_branch(repo: &Path, branch: &str) -> Result<()> {
    let out = Command::new("git")
        .args(["checkout", branch])
        .current_dir(repo)
        .output()
        .await?;
    if !out.status.success() {
        anyhow::bail!(
            "git checkout failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    Ok(())
}

pub async fn checkout_remote_branch(repo: &Path, remote_branch: &str, local_name: &str) -> Result<()> {
    let out = Command::new("git")
        .args(["checkout", "-b", local_name, remote_branch])
        .current_dir(repo)
        .output()
        .await?;
    if !out.status.success() {
        anyhow::bail!(
            "git checkout failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    Ok(())
}

pub async fn git_push(repo: &Path) -> Result<()> {
    let out = Command::new("git")
        .args(["push", "-u", "origin", "HEAD"])
        .current_dir(repo)
        .output()
        .await?;
    if !out.status.success() {
        anyhow::bail!("git push failed: {}", String::from_utf8_lossy(&out.stderr));
    }
    Ok(())
}

pub async fn git_pull(repo: &Path) -> Result<()> {
    let out = Command::new("git")
        .args(["pull"])
        .current_dir(repo)
        .output()
        .await?;
    if !out.status.success() {
        anyhow::bail!("git pull failed: {}", String::from_utf8_lossy(&out.stderr));
    }
    Ok(())
}

pub async fn git_fetch(repo: &Path) -> Result<()> {
    let out = Command::new("git")
        .args(["fetch"])
        .current_dir(repo)
        .output()
        .await?;
    if !out.status.success() {
        anyhow::bail!("git fetch failed: {}", String::from_utf8_lossy(&out.stderr));
    }
    Ok(())
}

pub async fn commit(repo: &Path, message: &str) -> Result<()> {
    let out = Command::new("git")
        .args(["commit", "-m", message])
        .current_dir(repo)
        .output()
        .await?;
    if !out.status.success() {
        anyhow::bail!(
            "git commit failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    Ok(())
}
