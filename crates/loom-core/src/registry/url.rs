/// Canonical representation of a git remote URL.
///
/// Normalized form: `{host}/{path}` where:
/// - host is lowercased (case-insensitive)
/// - path preserves case (case-sensitive on GitHub)
/// - `.git` suffix removed
/// - trailing slashes removed
/// - SSH `user@host:path` converted to `host/path`
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CanonicalUrl(String);

impl CanonicalUrl {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for CanonicalUrl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Normalize a git remote URL to canonical `{host}/{org}/{repo}` form.
///
/// Algorithm:
/// 1. Strip protocol (`ssh://`, `https://`, `git://`, `http://`)
/// 2. Strip `user@` prefix
/// 3. Convert SSH `:` separator to `/` (e.g., `git@github.com:org/repo`)
/// 4. Strip `.git` suffix
/// 5. Lowercase the host component
/// 6. Strip trailing slashes
/// 7. Preserve port numbers in host
pub fn normalize_url(url: &str) -> Option<CanonicalUrl> {
    let mut s = url.trim();

    if s.is_empty() {
        return None;
    }

    // 1. Strip protocol prefix
    for proto in &["ssh://", "https://", "http://", "git://"] {
        if let Some(rest) = s.strip_prefix(proto) {
            s = rest;
            break;
        }
    }

    // 2. Strip user@ prefix (e.g., git@github.com:org/repo)
    if let Some(at_pos) = s.find('@') {
        // Only strip if @ appears before the first / or : (i.e., it's a user prefix)
        let first_sep = s.find('/').unwrap_or(usize::MAX);
        let first_colon = s.find(':').unwrap_or(usize::MAX);
        if at_pos < first_sep && at_pos < first_colon {
            s = &s[at_pos + 1..];
        }
    }

    // 3. Convert SSH colon separator to slash
    // SSH format: `host:path` or `host:port/path`
    // We need to distinguish `github.com:org/repo` (SSH) from `gitlab.com:2222/org/repo` (port)
    if let Some(colon_pos) = s.find(':') {
        let after_colon = &s[colon_pos + 1..];
        // If the part after colon starts with a digit, it could be a port number
        // Check if it's `host:port/path` pattern
        if let Some(slash_after) = after_colon.find('/') {
            let maybe_port = &after_colon[..slash_after];
            if maybe_port.chars().all(|c| c.is_ascii_digit()) {
                // It's a port: keep host:port as the host component
                let host = &s[..colon_pos + 1 + slash_after];
                let path = &after_colon[slash_after + 1..];
                let canonical = format!("{}/{}", host.to_lowercase(), path.trim_end_matches('/'));
                let canonical = canonical.strip_suffix(".git").unwrap_or(&canonical);
                return Some(CanonicalUrl(canonical.to_string()));
            }
        }
        // Not a port — SSH colon separator: host:path -> host/path
        let host = &s[..colon_pos];
        let path = &s[colon_pos + 1..];
        let canonical = format!("{}/{}", host.to_lowercase(), path.trim_end_matches('/'));
        let canonical = canonical.strip_suffix(".git").unwrap_or(&canonical);
        return Some(CanonicalUrl(canonical.to_string()));
    }

    // 4-7. Standard URL form: host/path
    // Find host boundary (first /)
    if let Some(slash_pos) = s.find('/') {
        let host = &s[..slash_pos];
        let path = &s[slash_pos + 1..];
        let path = path.trim_end_matches('/');
        let path = path.strip_suffix(".git").unwrap_or(path);
        let canonical = format!("{}/{}", host.to_lowercase(), path);
        Some(CanonicalUrl(canonical))
    } else {
        // Just a host, no path — not a valid repo URL
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_https_url() {
        assert_eq!(
            normalize_url("https://github.com/org/repo")
                .unwrap()
                .as_str(),
            "github.com/org/repo"
        );
    }

    #[test]
    fn test_https_with_git_suffix() {
        assert_eq!(
            normalize_url("https://github.com/org/repo.git")
                .unwrap()
                .as_str(),
            "github.com/org/repo"
        );
    }

    #[test]
    fn test_ssh_url() {
        assert_eq!(
            normalize_url("git@github.com:org/repo.git")
                .unwrap()
                .as_str(),
            "github.com/org/repo"
        );
    }

    #[test]
    fn test_ssh_without_git_suffix() {
        assert_eq!(
            normalize_url("git@github.com:org/repo").unwrap().as_str(),
            "github.com/org/repo"
        );
    }

    #[test]
    fn test_ssh_protocol_url() {
        assert_eq!(
            normalize_url("ssh://git@github.com/org/repo.git")
                .unwrap()
                .as_str(),
            "github.com/org/repo"
        );
    }

    #[test]
    fn test_trailing_slash() {
        assert_eq!(
            normalize_url("https://github.com/org/repo/")
                .unwrap()
                .as_str(),
            "github.com/org/repo"
        );
    }

    #[test]
    fn test_host_case_insensitive() {
        assert_eq!(
            normalize_url("https://GitHub.COM/org/Repo")
                .unwrap()
                .as_str(),
            "github.com/org/Repo"
        );
    }

    #[test]
    fn test_gitlab_nested_groups() {
        assert_eq!(
            normalize_url("git@gitlab.internal:team/project/repo.git")
                .unwrap()
                .as_str(),
            "gitlab.internal/team/project/repo"
        );
    }

    #[test]
    fn test_ssh_and_https_same_canonical() {
        let ssh = normalize_url("git@github.com:org/repo.git").unwrap();
        let https = normalize_url("https://github.com/org/repo").unwrap();
        assert_eq!(ssh, https);
    }

    #[test]
    fn test_empty_url() {
        assert!(normalize_url("").is_none());
    }

    #[test]
    fn test_whitespace_url() {
        assert_eq!(
            normalize_url("  https://github.com/org/repo  ")
                .unwrap()
                .as_str(),
            "github.com/org/repo"
        );
    }

    #[test]
    fn test_git_protocol() {
        assert_eq!(
            normalize_url("git://github.com/org/repo.git")
                .unwrap()
                .as_str(),
            "github.com/org/repo"
        );
    }

    #[test]
    fn test_ssh_with_port() {
        assert_eq!(
            normalize_url("ssh://git@gitlab.example.com:2222/org/repo.git")
                .unwrap()
                .as_str(),
            "gitlab.example.com:2222/org/repo"
        );
    }
}
