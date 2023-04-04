use std::collections::HashSet;

use lazy_static::lazy_static;

use crate::util::errors::{forbidden, AppResult};

lazy_static! {
    static ref AUTHORIZED_ADMIN_USERS: HashSet<String> =
        parse_authorized_admin_users(dotenv::var("GH_ADMIN_USERS"));
}

const DEFAULT_ADMIN_USERS: [&str; 3] = ["carols10cents", "jtgeibel", "Turbo87"];

fn parse_authorized_admin_users(maybe_users: dotenv::Result<String>) -> HashSet<String> {
    match maybe_users {
        Ok(users) => users
            .split(|c: char| !(c.is_ascii_alphanumeric() || c == '-'))
            .filter(|user| !user.is_empty())
            .map(String::from)
            .collect(),
        Err(_err) => DEFAULT_ADMIN_USERS.into_iter().map(String::from).collect(),
    }
}

/// Return `Ok(())` if the given GitHub user name matches a known admin (set
/// either via the `$GH_ADMIN_USERS` environment variable, or the builtin
/// fallback list if that variable is unset), or a forbidden error otherwise.
pub fn is_authorized_admin(username: &str) -> AppResult<()> {
    // This hack is here to allow tests to have a consistent set of admin users
    // (in this case, just the contents of the `DEFAULT_ADMIN_USERS` constant
    // above).

    #[cfg(not(test))]
    fn check_username(username: &str) -> bool {
        AUTHORIZED_ADMIN_USERS.contains(username)
    }

    #[cfg(test)]
    fn check_username(username: &str) -> bool {
        DEFAULT_ADMIN_USERS.contains(&username)
    }

    if check_username(username) {
        Ok(())
    } else {
        Err(forbidden())
    }
}

#[cfg(test)]
mod tests {
    use std::io::{self, ErrorKind};

    use super::{is_authorized_admin, parse_authorized_admin_users, DEFAULT_ADMIN_USERS};

    #[test]
    fn test_is_authorized_admin() {
        assert_ok!(is_authorized_admin("Turbo87"));
        assert_err!(is_authorized_admin(""));
        assert_err!(is_authorized_admin("foo"));
    }

    #[test]
    fn test_parse_authorized_admin_users() {
        fn assert_authorized(input: dotenv::Result<&str>, expected: &[&str]) {
            assert_eq!(
                parse_authorized_admin_users(input.map(String::from)),
                expected.iter().map(|s| String::from(*s)).collect()
            );
        }

        assert_authorized(Ok(""), &[]);
        assert_authorized(Ok("foo"), &["foo"]);
        assert_authorized(Ok("foo, bar"), &["foo", "bar"]);
        assert_authorized(Ok("   foo  bar "), &["foo", "bar"]);
        assert_authorized(Ok("foo;bar"), &["foo", "bar"]);

        let not_found_error = dotenv::Error::Io(io::Error::new(ErrorKind::NotFound, "not found"));
        assert_authorized(Err(not_found_error), DEFAULT_ADMIN_USERS.as_slice());
    }
}
