use crate::models;

#[derive(Debug, Clone, Serialize)]
pub struct User {
    id: i32,
    avatar: Option<String>,
    username: String,
}

impl From<models::User> for User {
    fn from(value: models::User) -> Self {
        Self {
            id: value.id,
            avatar: value.gh_avatar,
            username: value.gh_login,
        }
    }
}
