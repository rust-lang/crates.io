use spdx::{Expression, ParseError};

const PARSE_MODE: spdx::ParseMode = spdx::ParseMode {
    allow_slash_as_or_operator: true,
    allow_imprecise_license_names: false,
    allow_postfix_plus_on_gpl: true,
    allow_deprecated: true,
};

pub fn parse_license_expr(s: &str) -> Result<Expression, ParseError> {
    Expression::parse_mode(s, PARSE_MODE)
}

#[cfg(test)]
mod tests {
    use super::parse_license_expr;
    use claims::{assert_err, assert_ok};

    #[test]
    fn licenses() {
        assert_ok!(parse_license_expr("MIT"));
        assert_ok!(parse_license_expr("MIT OR Apache-2.0"));
        assert_ok!(parse_license_expr("MIT/Apache-2.0"));
        assert_ok!(parse_license_expr("MIT AND Apache-2.0"));
        assert_ok!(parse_license_expr("MIT OR (Apache-2.0 AND MIT)"));
        assert_ok!(parse_license_expr("GPL-3.0+"));

        assert_err!(parse_license_expr("apache 2.0"));
    }
}
