use crate::util::errors::{cargo_err, AppResult};

const PARSE_MODE: spdx::ParseMode = spdx::ParseMode {
    allow_lower_case_operators: false,
    allow_slash_as_or_operator: true,
    allow_imprecise_license_names: false,
    allow_postfix_plus_on_gpl: true,
};

pub fn validate_license_expr(s: &str) -> AppResult<()> {
    spdx::Expression::parse_mode(s, PARSE_MODE).map_err(|_| {
        cargo_err("unknown or invalid license expression; see http://opensource.org/licenses for options, and http://spdx.org/licenses/ for their identifiers")
    })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validate_license_expr;

    #[test]
    fn licenses() {
        assert_ok!(validate_license_expr("MIT"));
        assert_ok!(validate_license_expr("MIT OR Apache-2.0"));
        assert_ok!(validate_license_expr("MIT/Apache-2.0"));
        assert_ok!(validate_license_expr("MIT AND Apache-2.0"));
        assert_ok!(validate_license_expr("MIT OR (Apache-2.0 AND MIT)"));
        assert_ok!(validate_license_expr("GPL-3.0+"));

        let error = assert_err!(validate_license_expr("apache 2.0")).to_string();
        assert!(error.starts_with("unknown or invalid license expression; see http"));
    }
}
