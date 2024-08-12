use googletest::{
    description::Description,
    matcher::{Matcher, MatcherBase, MatcherResult},
};
use http::StatusCode;

pub fn is_success() -> SuccessMatcher {
    SuccessMatcher
}

#[derive(MatcherBase)]
pub struct SuccessMatcher;

impl Matcher<StatusCode> for SuccessMatcher {
    fn matches(&self, actual: StatusCode) -> MatcherResult {
        actual.is_success().into()
    }

    fn describe(&self, matcher_result: MatcherResult) -> Description {
        match matcher_result {
            MatcherResult::Match => "is a success status code (200-299)".into(),
            MatcherResult::NoMatch => "isn't a success status code (200-299)".into(),
        }
    }
}
