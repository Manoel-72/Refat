pub const ENGINE_VERSION: &str = "V0.7-B";
pub const ENGINE_STATUS: &str = "EM TESTE";
pub const ENGINE_TITLE: &str = "RS2BR-Engine";

pub fn startup_message() -> String {
    format!("{} {} [{}]", ENGINE_TITLE, ENGINE_VERSION, ENGINE_STATUS)
}
