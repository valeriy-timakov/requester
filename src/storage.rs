use std::fs;
use std::path::{Path, PathBuf};
use directories::UserDirs;

#[derive(Debug, Clone)]
pub struct HttpRequest {
    pub method: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

impl HttpRequest {
    pub fn new() -> Self {
        Self {
            method: "GET".to_string(),
            url: "https://httpbin.org/get".to_string(),
            headers: Vec::new(),
            body: String::new(),
        }
    }

    pub fn to_http_string(&self) -> String {
        let mut s = format!("{} {}\n", self.method, self.url);
        for (k, v) in &self.headers {
            s.push_str(&format!("{}: {}\n", k, v));
        }
        s.push_str("\n");
        s.push_str(&self.body);
        s
    }

    pub fn from_http_string(s: &str) -> Result<Self, String> {
        let mut lines = s.lines();
        let first_line = lines.next().ok_or("Empty file")?;
        let parts: Vec<&str> = first_line.split_whitespace().collect();
        if parts.len() < 2 {
            return Err("Invalid first line: search for 'METHOD URL'".to_string());
        }
        let method = parts[0].to_uppercase();
        let url = parts[1..].join(" ");

        let mut headers = Vec::new();
        let mut body = String::new();
        let mut reading_body = false;

        for line in lines {
            if reading_body {
                body.push_str(line);
                body.push_str("\n");
            } else if line.trim().is_empty() {
                reading_body = true;
            } else if let Some((k, v)) = line.split_once(':') {
                headers.push((k.trim().to_string(), v.trim().to_string()));
            }
        }

        Ok(Self {
            method,
            url,
            headers,
            body: body.trim_end().to_string(),
        })
    }
}

pub fn get_base_dir() -> PathBuf {
    UserDirs::new()
        .map(|dirs| dirs.home_dir().join("requester"))
        .unwrap_or_else(|| PathBuf::from("requester"))
}

pub fn ensure_base_dir() -> std::io::Result<()> {
    let path = get_base_dir();
    if !path.exists() {
        fs::create_dir_all(path)?;
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum FileNode {
    File { name: String, path: PathBuf },
    Folder { name: String, path: PathBuf, children: Vec<FileNode> },
}

pub fn scan_directory() -> FileNode {
    let base_path = get_base_dir();
    if !base_path.exists() {
        let _ = fs::create_dir_all(&base_path);
    }
    
    build_tree(&base_path)
}

fn build_tree(path: &Path) -> FileNode {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("requester").to_string();
    if path.is_dir() {
        let mut children = Vec::new();
        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_dir() || p.extension().map_or(false, |ext| ext == "req") {
                    children.push(build_tree(&p));
                }
            }
        }
        // Sort folders first, then files
        children.sort_by(|a, b| {
            match (a, b) {
                (FileNode::Folder { .. }, FileNode::File { .. }) => std::cmp::Ordering::Less,
                (FileNode::File { .. }, FileNode::Folder { .. }) => std::cmp::Ordering::Greater,
                _ => a.name().cmp(b.name()),
            }
        });
        FileNode::Folder { name, path: path.to_path_buf(), children }
    } else {
        FileNode::File { name, path: path.to_path_buf() }
    }
}

impl FileNode {
    pub fn name(&self) -> &str {
        match self {
            FileNode::File { name, .. } => name,
            FileNode::Folder { name, .. } => name,
        }
    }
    
    pub fn path(&self) -> &Path {
        match self {
            FileNode::File { path, .. } => path,
            FileNode::Folder { path, .. } => path,
        }
    }
}

pub fn load_request(path: &Path) -> Result<HttpRequest, String> {
    let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
    HttpRequest::from_http_string(&content)
}

pub fn save_request(path: &Path, req: &HttpRequest) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, req.to_http_string())
}
