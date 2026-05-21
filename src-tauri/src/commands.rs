use serde::Serialize;

#[derive(Serialize)]
pub struct HelloReply {
    pub message: String,
    pub version: &'static str,
}

#[tauri::command]
pub fn hello(name: Option<String>) -> HelloReply {
    let who = name.unwrap_or_else(|| "world".to_string());
    HelloReply {
        message: format!("Hello, {who}! SnipTeX backend is alive."),
        version: env!("CARGO_PKG_VERSION"),
    }
}
