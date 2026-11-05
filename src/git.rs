use std::fs;
use std::path::{Path, PathBuf};

/// What `HEAD` points at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Head {
    Branch(String),
    /// The short SHA of the checked-out commit.
    Detached(String),
}

/// Reads `HEAD` without spawning a subprocess.
pub fn read_head(start_path: &Path) -> Option<Head> {
    let (_, git_dir) = find_git_dir(start_path)?;
    let content = fs::read_to_string(git_dir.join("HEAD")).ok()?;
    let line = content.trim();

    if let Some(branch) = line.strip_prefix("ref: refs/heads/") {
        Some(Head::Branch(branch.to_string()))
    } else if line.len() >= 7 {
        Some(Head::Detached(line[..7].to_string()))
    } else {
        None
    }
}

/// Resolves the repository root (the directory containing the `.git` entry)
/// by walking upward from `start_path`. Stable regardless of which
/// subdirectory of the repository `start_path` is in, so callers can use it
/// as a consistent cache key.
pub fn find_repo_root(start_path: &Path) -> Option<PathBuf> {
    let (root, _) = find_git_dir(start_path)?;
    Some(root)
}

fn find_git_dir(start_path: &Path) -> Option<(PathBuf, PathBuf)> {
    let mut current = if start_path.is_file() {
        start_path.parent()?
    } else {
        start_path
    };

    loop {
        let dot_git = current.join(".git");
        if dot_git.is_dir() {
            return Some((current.to_path_buf(), dot_git));
        } else if dot_git.is_file()
            && let Ok(content) = fs::read_to_string(&dot_git)
            && let Some(target) = content.trim().strip_prefix("gitdir:")
        {
            let target_path = PathBuf::from(target.trim());
            let git_dir = if target_path.is_absolute() {
                target_path
            } else {
                current.join(target_path)
            };
            return Some((current.to_path_buf(), git_dir));
        }

        match current.parent() {
            Some(p) if p != current => current = p,
            _ => break,
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_non_git_directory() {
        let temp_dir = tempfile::tempdir().unwrap();
        let head = read_head(temp_dir.path());
        assert!(head.is_none());
    }

    fn repo_with_head(head: &str) -> tempfile::TempDir {
        let temp_dir = tempfile::tempdir().unwrap();
        let dot_git = temp_dir.path().join(".git");
        fs::create_dir_all(&dot_git).unwrap();
        fs::write(dot_git.join("HEAD"), head).unwrap();
        temp_dir
    }

    #[test]
    fn test_head_on_a_branch_names_the_branch() {
        let repo = repo_with_head("ref: refs/heads/feature-awesome\n");

        let head = read_head(repo.path());

        assert_eq!(head, Some(Head::Branch("feature-awesome".to_string())));
    }

    #[test]
    fn test_a_detached_head_gives_the_short_sha() {
        let repo = repo_with_head("d58460183921abc456\n");

        let head = read_head(repo.path());

        assert_eq!(head, Some(Head::Detached("d584601".to_string())));
    }

    #[test]
    fn test_find_repo_root_is_stable_from_nested_subdirectory() {
        let temp_dir = tempfile::tempdir().unwrap();
        let dot_git = temp_dir.path().join(".git");
        let nested = temp_dir.path().join("a").join("b");
        fs::create_dir_all(&dot_git).unwrap();
        fs::create_dir_all(&nested).unwrap();

        let root_from_top = find_repo_root(temp_dir.path());
        let root_from_nested = find_repo_root(&nested);

        assert_eq!(root_from_top, root_from_nested);
        assert_eq!(root_from_top.unwrap(), temp_dir.path());
    }
}
