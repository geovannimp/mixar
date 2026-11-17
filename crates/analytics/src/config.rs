pub const APP_VERSION: &str = env!("MIXAR_APP_VERSION");
pub const APP_LAUNCHED_EVENT: &str = "app launched";

const DEFAULT_TOKEN: &str = "";

pub fn token() -> &'static str {
    option_env!("MIXAR_POSTHOG_TOKEN").unwrap_or(DEFAULT_TOKEN)
}

pub fn capture_url() -> &'static str {
    "https://eu.i.posthog.com/batch/"
}
