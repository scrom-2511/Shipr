use crate::{
    app_errors::AppError,
    app_types::DeployDetails,
    config::project_default_config::get_default_config,
    utils::{detect_project_type::detect_project_type, run_script::run_script_vm},
};

pub struct PullBuildWorker;

impl PullBuildWorker {
    pub fn new() -> Self {
        Self {}
    }

    fn extract_repo_name(&self, url: &str) -> Result<String, AppError> {
        let url = url.trim();

        let last_part = url.split('/').last().ok_or(AppError::InvalidGitUrl)?;

        let repo_name = last_part.strip_suffix(".git").unwrap_or(last_part);

        Ok(repo_name.to_string())
    }
}
