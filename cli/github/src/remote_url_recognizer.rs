//! The GitHub implementation of `collaboration`'s `RemoteUrlRecognizer`
//! port.

use collaboration::{OwnerRepo, RemoteUrlRecognizer};

/// Recognizes `github.com` remote URLs, in any of the shapes `git` itself
/// accepts for `origin`: HTTPS, the `git@host:owner/repo` SCP-like form,
/// and explicit `ssh://`/`git://`.
pub struct GitHubRemoteUrlRecognizer;

impl RemoteUrlRecognizer for GitHubRemoteUrlRecognizer {
    fn parse(&self, url: &str) -> Option<OwnerRepo> {
        let path = strip_scp_like(url)
            .or_else(|| strip_scheme(url, "https://github.com/"))
            .or_else(|| strip_scheme(url, "http://github.com/"))
            .or_else(|| strip_scheme(url, "ssh://git@github.com/"))
            .or_else(|| strip_scheme(url, "git://github.com/"))?;
        owner_repo_from_path(path)
    }
}

fn strip_scp_like(url: &str) -> Option<&str> {
    url.strip_prefix("git@github.com:")
}

fn strip_scheme<'a>(url: &'a str, prefix: &str) -> Option<&'a str> {
    url.strip_prefix(prefix)
}

fn owner_repo_from_path(path: &str) -> Option<OwnerRepo> {
    let path = path.strip_suffix(".git").unwrap_or(path);
    let mut segments = path.split('/');
    let owner = segments.next()?;
    let repo = segments.next()?;
    if owner.is_empty() || repo.is_empty() || segments.next().is_some() {
        return None;
    }
    Some(OwnerRepo {
        owner: owner.to_owned(),
        repo: repo.to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::GitHubRemoteUrlRecognizer;
    use collaboration::{OwnerRepo, RemoteUrlRecognizer};

    fn candidate() -> OwnerRepo {
        OwnerRepo {
            owner: "atomicinnovation".to_owned(),
            repo: "accelerator".to_owned(),
        }
    }

    #[test]
    fn a_https_url_with_the_git_suffix_is_recognized() {
        let result = GitHubRemoteUrlRecognizer
            .parse("https://github.com/atomicinnovation/accelerator.git");
        assert_eq!(result, Some(candidate()));
    }

    #[test]
    fn a_https_url_without_the_git_suffix_is_recognized() {
        let result = GitHubRemoteUrlRecognizer
            .parse("https://github.com/atomicinnovation/accelerator");
        assert_eq!(result, Some(candidate()));
    }

    #[test]
    fn a_plain_http_url_is_recognized() {
        let result = GitHubRemoteUrlRecognizer
            .parse("http://github.com/atomicinnovation/accelerator.git");
        assert_eq!(result, Some(candidate()));
    }

    #[test]
    fn an_scp_like_ssh_url_is_recognized() {
        let result = GitHubRemoteUrlRecognizer
            .parse("git@github.com:atomicinnovation/accelerator.git");
        assert_eq!(result, Some(candidate()));
    }

    #[test]
    fn an_explicit_ssh_url_is_recognized() {
        let result = GitHubRemoteUrlRecognizer
            .parse("ssh://git@github.com/atomicinnovation/accelerator.git");
        assert_eq!(result, Some(candidate()));
    }

    #[test]
    fn a_git_protocol_url_is_recognized() {
        let result = GitHubRemoteUrlRecognizer
            .parse("git://github.com/atomicinnovation/accelerator.git");
        assert_eq!(result, Some(candidate()));
    }

    #[test]
    fn a_url_for_a_different_host_is_not_recognized() {
        let result = GitHubRemoteUrlRecognizer
            .parse("https://gitlab.com/atomicinnovation/accelerator.git");
        assert_eq!(result, None);
    }

    #[test]
    fn a_url_missing_a_repo_segment_is_not_recognized() {
        let result = GitHubRemoteUrlRecognizer
            .parse("https://github.com/atomicinnovation");
        assert_eq!(result, None);
    }

    #[test]
    fn a_url_with_extra_path_segments_is_not_recognized() {
        let result = GitHubRemoteUrlRecognizer
            .parse("https://github.com/atomicinnovation/accelerator/pulls");
        assert_eq!(result, None);
    }
}
