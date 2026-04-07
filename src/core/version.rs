pub const ENGINE_VERSION: &str = concat!("V", env!("CARGO_PKG_VERSION"));
pub const ENGINE_STATUS: &str = concat!("V", env!("CARGO_PKG_VERSION"), " - V0.9.4 - gameplay layer");
pub const ENGINE_TITLE: &str = "RS2BR-Engine";

pub fn startup_message() -> String {
    format!("{} {} [{}]", ENGINE_TITLE, ENGINE_VERSION, ENGINE_STATUS)
}
