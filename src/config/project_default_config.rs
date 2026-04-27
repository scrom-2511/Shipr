use crate::utils::detect_project_type::ProjectType;

pub struct ProjectDefaultConfig {
    pub install_commands: Vec<&'static str>,
    pub build_commands: Vec<&'static str>,
    pub run_command: Option<&'static str>,
    pub dist_dir: &'static str,
    pub home_dir: &'static str,
}

pub fn node_default_config() -> ProjectDefaultConfig {
    ProjectDefaultConfig {
        install_commands: vec!["npm install"],
        build_commands: vec!["npx tsc", "npm run build"],
        run_command: Some("node index.js"),
        dist_dir: "dist",
        home_dir: ".",
    }
}

pub fn react_default_config() -> ProjectDefaultConfig {
    ProjectDefaultConfig {
        install_commands: vec!["npm install"],
        build_commands: vec!["npm run build"],
        run_command: None,
        dist_dir: "dist",
        home_dir: ".",
    }
}

pub fn html_default_config() -> ProjectDefaultConfig {
    ProjectDefaultConfig {
        install_commands: vec![],
        build_commands: vec![],
        run_command: Some("npx serve ."),
        dist_dir: "dist",
        home_dir: ".",
    }
}

pub fn get_default_config(project_type: ProjectType) -> ProjectDefaultConfig {
    match project_type {
        ProjectType::Node => node_default_config(),
        ProjectType::React => react_default_config(),
        ProjectType::Html => html_default_config(),
        ProjectType::Unknown => panic!("Unknown project type"),
        _ => panic!("Unknown project type"),
    }
}
