use std::{env, fmt, fs, path::Path};

#[derive(Clone)]
pub enum ProjectType {
    Html,
    Rust,
    React,
    Node,
    Unknown,
}

impl fmt::Display for ProjectType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            ProjectType::Html => "html",
            ProjectType::Rust => "rust",
            ProjectType::React => "react",
            ProjectType::Node => "node",
            ProjectType::Unknown => "unknown",
        };
        write!(f, "{}", s)
    }
}

pub fn detect_project_type(path: &str) -> ProjectType {
    let html_path = format!("{}/dist/index.html", path);
    let package_json_path = format!("{}/package.json", path);
    let cargo_toml_path = format!("{}/Cargo.toml", path);

    if Path::new(&html_path).exists() {
        return ProjectType::Html;
    }

    if Path::new(&package_json_path).exists() {
        let package_json_str = fs::read_to_string(&package_json_path).unwrap();
        let package_json = serde_json::from_str::<serde_json::Value>(&package_json_str).unwrap();

        if package_json["dependencies"].get("react").is_some() {
            return ProjectType::React;
        }

        if package_json["dependencies"].get("express").is_some() {
            return ProjectType::Node;
        }
    }

    if Path::new(&cargo_toml_path).exists() {
        return ProjectType::Rust;
    }

    ProjectType::Unknown
}
