/// Builds the tags shared by Datadog submissions from this application.
pub fn common_tags(domain_name: &str) -> Vec<String> {
    let environment = match domain_name {
        "staging.crates.io" => "staging",
        _ => "prod",
    };

    vec![format!("env:{environment}"), "service:crates_io".into()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_common_tags() {
        let staging = common_tags("staging.crates.io");
        insta::assert_json_snapshot!(staging, @r#"
        [
          "env:staging",
          "service:crates_io"
        ]
        "#);

        let production = common_tags("crates.io");
        insta::assert_json_snapshot!(production, @r#"
        [
          "env:prod",
          "service:crates_io"
        ]
        "#);
    }
}
