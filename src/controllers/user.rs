pub mod email_notifications;
pub mod email_verification;
pub mod emails;
pub mod me;
pub mod other;
pub mod update;

#[allow(deprecated)]
pub use email_verification::{resend_email_verification, resend_email_verification_all};
pub use emails::{create_email, delete_email};
pub use update::update_user;
