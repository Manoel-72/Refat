pub const ENGINE_VERSION: &str = "V0.9.0";
pub const ENGINE_STATUS: &str = "V0.9 - MVP";
pub const ENGINE_TITLE: &str = "RS2BR-Engine";

pub fn startup_message() -> String {
    format!("{} {} [{}]", ENGINE_TITLE, ENGINE_VERSION, ENGINE_STATUS)
}
