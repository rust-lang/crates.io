#[derive(Debug, Clone, Serialize)]
pub struct User {
    avatar: Option<String>,
    username: String,
}

impl User {
    pub fn new(username: String, avatar: Option<String>) -> Self {
        Self { avatar, username }
    }
}
