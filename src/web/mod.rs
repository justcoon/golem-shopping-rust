use golem_rust::agentic::UnstructuredBinary;
use golem_rust::{agent_definition, agent_implementation, endpoint};

fn mime_type_for_path(path: &str) -> &'static str {
    if path.ends_with(".js") {
        "text/javascript; charset=utf-8"
    } else if path.ends_with(".css") {
        "text/css; charset=utf-8"
    } else if path.ends_with(".html") {
        "text/html; charset=utf-8"
    } else if path.ends_with(".json") {
        "application/json"
    } else if path.ends_with(".svg") {
        "image/svg+xml"
    } else if path.ends_with(".png") {
        "image/png"
    } else if path.ends_with(".ico") {
        "image/x-icon"
    } else {
        "application/octet-stream"
    }
}

fn get_embedded_file(relative_path: &str) -> Option<(&'static [u8], &'static str)> {
    match relative_path {
        "assistant.html" => Some((
            include_bytes!("../../web/assistant.html"),
            "text/html; charset=utf-8",
        )),
        "manager.html" => Some((
            include_bytes!("../../web/manager.html"),
            "text/html; charset=utf-8",
        )),
        "assets/assistant.css" => Some((
            include_bytes!("../../web/assets/assistant.css"),
            "text/css; charset=utf-8",
        )),
        "assets/assistant.js" => Some((
            include_bytes!("../../web/assets/assistant.js"),
            "text/javascript; charset=utf-8",
        )),
        "assets/chat-common.js" => Some((
            include_bytes!("../../web/assets/chat-common.js"),
            "text/javascript; charset=utf-8",
        )),
        "assets/manager.js" => Some((
            include_bytes!("../../web/assets/manager.js"),
            "text/javascript; charset=utf-8",
        )),
        _ => None,
    }
}

fn serve_file(
    relative_path: &str,
    fallback_mime: &'static str,
) -> Option<UnstructuredBinary<String>> {
    let clean_path = relative_path.trim_start_matches('/');
    let fs_path = format!("/web/{clean_path}");

    if let Ok(bytes) = std::fs::read(&fs_path) {
        let mime = if fallback_mime.is_empty() {
            mime_type_for_path(clean_path)
        } else {
            fallback_mime
        };
        return Some(UnstructuredBinary::Inline {
            data: bytes,
            mime_type: mime.to_string(),
        });
    }

    if let Some((embedded_bytes, embedded_mime)) = get_embedded_file(clean_path) {
        return Some(UnstructuredBinary::Inline {
            data: embedded_bytes.to_vec(),
            mime_type: embedded_mime.to_string(),
        });
    }

    None
}

#[agent_definition(ephemeral, mount = "/assistant")]
pub trait WebAssistant {
    fn new() -> Self;

    #[endpoint(get = "/")]
    fn index(&self) -> Option<UnstructuredBinary<String>>;

    #[endpoint(get = "/assets/{*path}")]
    fn asset(&self, path: String) -> Option<UnstructuredBinary<String>>;
}

pub struct WebAssistantImpl;

#[agent_implementation]
impl WebAssistant for WebAssistantImpl {
    fn new() -> Self {
        WebAssistantImpl
    }

    fn index(&self) -> Option<UnstructuredBinary<String>> {
        serve_file("assistant.html", "text/html; charset=utf-8")
    }

    fn asset(&self, path: String) -> Option<UnstructuredBinary<String>> {
        serve_file(&format!("assets/{path}"), "")
    }
}

#[agent_definition(ephemeral, mount = "/manager")]
pub trait WebManager {
    fn new() -> Self;

    #[endpoint(get = "/")]
    fn index(&self) -> Option<UnstructuredBinary<String>>;

    #[endpoint(get = "/assets/{*path}")]
    fn asset(&self, path: String) -> Option<UnstructuredBinary<String>>;
}

pub struct WebManagerImpl;

#[agent_implementation]
impl WebManager for WebManagerImpl {
    fn new() -> Self {
        WebManagerImpl
    }

    fn index(&self) -> Option<UnstructuredBinary<String>> {
        serve_file("manager.html", "text/html; charset=utf-8")
    }

    fn asset(&self, path: String) -> Option<UnstructuredBinary<String>> {
        serve_file(&format!("assets/{path}"), "")
    }
}
