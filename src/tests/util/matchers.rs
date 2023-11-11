use googletest::matcher::{Matcher, MatcherResult};
use http::StatusCode;

pub fn is_success() -> SuccessMatcher {
    SuccessMatcher
}

pub struct SuccessMatcher;

impl Matcher for SuccessMatcher {
    type ActualT = StatusCode;

    fn matches(&self, actual: &Self::ActualT) -> MatcherResult {
        actual.is_success().into()
    }

    fn describe(&self, matcher_result: MatcherResult) -> String {
        match matcher_result {
            MatcherResult::Match => "is a success status code (200-299)".into(),
            MatcherResult::NoMatch => "isn't a success status code (200-299)".into(),
        }
    }
}

pub fn is_redirection() -> RedirectionMatcher {
    RedirectionMatcher
}

pub struct RedirectionMatcher;

impl Matcher for RedirectionMatcher {
    type ActualT = StatusCode;

    fn matches(&self, actual: &Self::ActualT) -> MatcherResult {
        actual.is_redirection().into()
    }

    fn describe(&self, matcher_result: MatcherResult) -> String {
        match matcher_result {
            MatcherResult::Match => "is a redirection status code (300-399)".into(),
            MatcherResult::NoMatch => "isn't a redirection status code (300-399)".into(),
        }
    }
}
