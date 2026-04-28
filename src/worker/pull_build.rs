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

    fn get_project_path(&self, deploy_details: &DeployDetails) -> Result<String, AppError> {
        let repo_name = self.extract_repo_name(&deploy_details.url)?;

        if deploy_details.home_dir.is_empty() {
            Ok(format!("/root/{}", repo_name))
        } else {
            Ok(format!("/root/{}/{}", repo_name, deploy_details.home_dir))
        }
    }

    async fn pull(&self, deploy_details: &DeployDetails) -> Result<(), AppError> {
        let git_clone_cmd = format!("cd /root && git clone {}", deploy_details.url);

        run_script_vm(vec![&git_clone_cmd])?;

        Ok(())
    }

    async fn install(&self, deploy_details: &DeployDetails) -> Result<(), AppError> {
        let project_path = self.get_project_path(deploy_details)?;

        if !deploy_details.install_commands.is_empty() {
            let install_cmd = deploy_details.install_commands.join(" && ");

            let final_cmd = format!("cd {} && {}", project_path, install_cmd);

            run_script_vm(vec![&final_cmd])?;
            return Ok(());
        }

        let project_type = detect_project_type(&project_path);
        let config = get_default_config(project_type);

        let final_cmd = format!(
            "cd {} && {}",
            project_path,
            config.install_commands.join(" && ")
        );

        run_script_vm(vec![&final_cmd])?;

        Ok(())
    }

    async fn build(&self, deploy_details: &DeployDetails) -> Result<(), AppError> {
        let project_path = self.get_project_path(deploy_details)?;
        println!("{}", project_path);

        if !deploy_details.build_commands.is_empty() {
            let build_cmd = deploy_details.build_commands.join(" && ");

            let final_cmd = format!("cd {} && {}", project_path, build_cmd);

            run_script_vm(vec![&final_cmd])?;
            return Ok(());
        }

        let project_type = detect_project_type(&project_path);
        let config = get_default_config(project_type);

        let final_cmd = format!(
            "cd {} && {}",
            project_path,
            config.build_commands.join(" && ")
        );

        run_script_vm(vec![&final_cmd])?;

        let zip_cmd = format!(
            "zip -r {}.zip {}/{}",
            "dist",
            self.extract_repo_name(&deploy_details.url)?,
            deploy_details.dist_dir
        );

        println!("{}", zip_cmd);

        run_script_vm(vec![&zip_cmd])?;

        let upload_cmd = format!(
            "curl -X PUT -T {}.zip '{}'",
            "dist", deploy_details.presigned_url
        );

        println!("{}", upload_cmd);

        run_script_vm(vec![&upload_cmd])?;

        Ok(())
    }

    pub async fn pull_build(&self, deploy_details: &DeployDetails) -> Result<(), AppError> {
        self.pull(deploy_details).await?;
        self.install(deploy_details).await?;
        self.build(deploy_details).await?;

        Ok(())
    }
}
