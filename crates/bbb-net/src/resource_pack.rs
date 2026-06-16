use bbb_protocol::packets::{ResourcePackPush, ResourcePackResponseAction};

pub(crate) fn response_action_for_push(update: &ResourcePackPush) -> ResourcePackResponseAction {
    if is_allowed_resource_pack_url(&update.url) {
        ResourcePackResponseAction::Declined
    } else {
        ResourcePackResponseAction::InvalidUrl
    }
}

fn is_allowed_resource_pack_url(url: &str) -> bool {
    if url
        .chars()
        .any(|ch| ch.is_ascii_control() || ch.is_whitespace())
    {
        return false;
    }
    let Some((scheme, rest)) = url.split_once(':') else {
        return false;
    };
    if scheme.is_empty() || rest.is_empty() {
        return false;
    }
    let mut chars = scheme.chars();
    if !chars.next().is_some_and(|ch| ch.is_ascii_alphabetic()) {
        return false;
    }
    if !chars.all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '+' | '-' | '.')) {
        return false;
    }
    matches!(scheme.to_ascii_lowercase().as_str(), "http" | "https")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resource_pack_response_follows_vanilla_url_scheme_gate() {
        assert!(is_allowed_resource_pack_url(
            "https://example.invalid/pack.zip"
        ));
        assert!(is_allowed_resource_pack_url(
            "http://example.invalid/pack.zip"
        ));
        assert!(!is_allowed_resource_pack_url(
            "ftp://example.invalid/pack.zip"
        ));
        assert!(!is_allowed_resource_pack_url("https://bad host/pack.zip"));
        assert!(!is_allowed_resource_pack_url("not-a-url"));
    }
}
