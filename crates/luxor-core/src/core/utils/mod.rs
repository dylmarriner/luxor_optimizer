pub fn redact_path(path: &str, redact_usernames: bool) -> String {
    if !redact_usernames {
        return path.to_string();
    }
    let re = regex::Regex::new(r"/home/[^/]+/").expect("valid regex");
    re.replace_all(path, "/home/<redacted>/").to_string()
}
