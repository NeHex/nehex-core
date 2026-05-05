pub fn normalize_comment_target_type(target_type: &str) -> String {
    target_type.trim().to_lowercase()
}

pub fn build_comment_sync_token(target_type: &str, target_id: i64) -> String {
    format!(
        "{}:{}",
        normalize_comment_target_type(target_type),
        target_id.max(1)
    )
}

pub fn build_comment_target_path(
    target_type: &str,
    target_id: i64,
    page_key: Option<&str>,
) -> String {
    let normalized_type = normalize_comment_target_type(target_type);
    let normalized_id = target_id.max(1);

    match normalized_type.as_str() {
        "article" => format!("/article/{normalized_id}"),
        "album" => format!("/album/{normalized_id}"),
        "daily" => format!("/daily/{normalized_id}"),
        "project" => format!("/project/{normalized_id}"),
        "friend_page" => "/friends".to_string(),
        "singlepage" => {
            let normalized_page_key = page_key
                .map(|value| value.trim().trim_matches('/'))
                .filter(|value| !value.is_empty());
            if let Some(value) = normalized_page_key {
                format!("/{value}")
            } else {
                format!("/page/{normalized_id}")
            }
        }
        _ => "/".to_string(),
    }
}

pub fn join_site_url(site_url: &str, path: &str) -> String {
    let normalized_path = if path.trim().is_empty() {
        "/".to_string()
    } else {
        format!("/{}", path.trim().trim_start_matches('/'))
    };

    if site_url.trim().is_empty() {
        normalized_path
    } else {
        format!(
            "{}{}",
            site_url.trim().trim_end_matches('/'),
            normalized_path
        )
    }
}

pub fn with_comment_anchor(target_url: &str, comment_id: i64) -> String {
    if target_url.trim().is_empty() {
        String::new()
    } else {
        format!("{}#comment-{}", target_url, comment_id.max(1))
    }
}

#[cfg(test)]
mod tests {
    use super::{
        build_comment_sync_token, build_comment_target_path, join_site_url, with_comment_anchor,
    };

    #[test]
    fn comment_sync_token_is_normalized() {
        assert_eq!(build_comment_sync_token(" Daily ", 0), "daily:1");
        assert_eq!(build_comment_sync_token("PROJECT", 42), "project:42");
    }

    #[test]
    fn comment_target_path_covers_public_routes() {
        assert_eq!(build_comment_target_path("article", 7, None), "/article/7");
        assert_eq!(build_comment_target_path("album", 8, None), "/album/8");
        assert_eq!(build_comment_target_path("daily", 9, None), "/daily/9");
        assert_eq!(build_comment_target_path("project", 10, None), "/project/10");
        assert_eq!(
            build_comment_target_path("friend_page", 0, None),
            "/friends"
        );
    }

    #[test]
    fn singlepage_target_path_prefers_page_key() {
        assert_eq!(
            build_comment_target_path("singlepage", 11, Some(" about/me ")),
            "/about/me"
        );
        assert_eq!(build_comment_target_path("singlepage", 11, Some("///")), "/page/11");
        assert_eq!(build_comment_target_path("singlepage", 11, None), "/page/11");
    }

    #[test]
    fn comment_target_url_helpers_build_full_direct_link() {
        let url = join_site_url("https://example.com/", "/daily/12");
        assert_eq!(url, "https://example.com/daily/12");
        assert_eq!(
            with_comment_anchor(&url, 34),
            "https://example.com/daily/12#comment-34"
        );
    }
}
