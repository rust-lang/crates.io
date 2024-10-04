use ::dialoguer::{theme::Theme, Confirm};

pub fn confirm(msg: &str) -> dialoguer::Result<bool> {
    Confirm::with_theme(&CustomTheme)
        .with_prompt(msg)
        .default(false)
        .wait_for_newline(true)
        .interact()
}

#[derive(Debug, Copy, Clone)]
struct CustomTheme;

impl Theme for CustomTheme {
    fn format_confirm_prompt(
        &self,
        f: &mut dyn std::fmt::Write,
        prompt: &str,
        default: Option<bool>,
    ) -> std::fmt::Result {
        if !prompt.is_empty() {
            write!(f, "{} ", &prompt)?;
        }
        match default {
            None => write!(f, "[y/n] ")?,
            Some(true) => write!(f, "[Y/n] yes")?,
            Some(false) => write!(f, "[y/N] no")?,
        }
        Ok(())
    }
}
