use super::templating::components::{DateTime, User};

#[derive(Serialize)]
pub struct CrateVersion {
    pub id: i32,
    pub name: String,
    pub num: String,
    pub created_at: DateTime,
    pub publisher: User,
}
