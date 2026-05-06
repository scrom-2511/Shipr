use std::{fs, thread::sleep, time::Duration};

use tokio::net::TcpStream;

use crate::{
    app_errors::AppError,
    app_types::{DeployDetails, RedeployDetails, RunDetails},
    config::{app_config::get_worker_dir, project_default_config::get_default_config},
    infra::{
        detect::detect_project_type,
        process::{run_script, run_script_bg},
    },
    worker::api::githubapp::GithubApp,
};

pub struct JobExecuter;

impl JobExecuter {
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
        let repo_name = deploy_details.project_id.to_owned();

        if deploy_details.home_dir.is_empty() {
            Ok(format!("/root/{}", repo_name))
        } else {
            Ok(format!("/root/{}/{}", repo_name, deploy_details.home_dir))
        }
    }

    async fn pull(&self, deploy_details: &DeployDetails) -> Result<(), AppError> {
        let mut github_app = GithubApp::new(
            deploy_details.access_token.to_owned(),
            deploy_details.owner.to_owned(),
            deploy_details.repo.to_owned(),
        );

        let tarball_url = github_app.get_tarball_url().await?;
        println!("tarball url is: {}", tarball_url);

        let git_clone_cmd = format!(
            "curl -Lo {}.tar.gz {} -H 'Accept: application/vnd.github.v3+json' -H 'Authorization: token {}'",
            deploy_details.project_id, tarball_url, deploy_details.access_token
        );

        println!("git clone command is: {}", git_clone_cmd);

        let extract_cmd = format!("tar -xzf {}.tar.gz", deploy_details.project_id);

        println!("extract command is: {}", extract_cmd);

        let rename_cmd = format!(
            "mv {}-* {}",
            deploy_details.project_id, deploy_details.project_id
        );

        println!("rename command is: {}", rename_cmd);

        run_script(
            vec![&git_clone_cmd, &extract_cmd, &rename_cmd],
            get_worker_dir(),
        )?;

        Ok(())
    }

    async fn install(&self, deploy_details: &DeployDetails) -> Result<(), AppError> {
        let project_path = self.get_project_path(deploy_details)?;

        println!("project path is: {}", project_path);

        if !deploy_details.install_commands.is_empty() {
            let install_cmd = deploy_details.install_commands.join(" && ");

            let final_cmd = format!("cd {} && {}", project_path, install_cmd);

            run_script(vec![&final_cmd], get_worker_dir())?;

            return Ok(());
        }

        let project_type = detect_project_type(&project_path);

        let config = get_default_config(project_type);

        let final_cmd = format!(
            "cd {} && {}",
            project_path,
            config.install_commands.join(" && ")
        );

        println!("final command is: {}", final_cmd);

        run_script(vec![&final_cmd], get_worker_dir())?;

        Ok(())
    }

    async fn build(&self, deploy_details: &DeployDetails) -> Result<(), AppError> {
        let project_path = self.get_project_path(deploy_details)?;

        if !deploy_details.build_commands.is_empty() {
            let build_cmd = deploy_details.build_commands.join(" && ");

            let final_cmd = format!("cd {} && {}", project_path, build_cmd);

            run_script(vec![&final_cmd], get_worker_dir())?;
            return Ok(());
        }

        let project_type = detect_project_type(&project_path);
        let config = get_default_config(project_type);

        let final_cmd = format!(
            "cd {} && {}",
            project_path,
            config.build_commands.join(" && ")
        );

        println!("final command is: {}", final_cmd);

        run_script(vec![&final_cmd], get_worker_dir())?;

        let zip_cmd = format!(
            "cd /root/{}/{} && zip -r /root/{}.zip . /root/job.json",
            deploy_details.project_id, deploy_details.dist_dir, deploy_details.project_id
        );

        println!("zip command is: {}", zip_cmd);

        run_script(vec![&zip_cmd], get_worker_dir())?;

        let upload_cmd = format!(
            "curl -X PUT -T {}.zip '{}'",
            deploy_details.project_id, deploy_details.presigned_upload_url
        );

        run_script(vec![&upload_cmd], get_worker_dir())?;

        Ok(())
    }

    pub async fn execute(&self, deploy_details: &DeployDetails) -> Result<(), AppError> {
        self.pull(deploy_details).await?;

        self.install(deploy_details).await?;

        self.build(deploy_details).await?;

        Ok(())
    }

    pub async fn run(&self, run_details: &RunDetails) -> Result<(), AppError> {
        let port_exists = TcpStream::connect("172.16.0.2:3000").await.is_ok();

        if port_exists {
            return Ok(());
        }

        let project_id = &run_details.project_id;

        let download_cmd = format!(
            "curl -o {}.zip '{}'",
            project_id, run_details.presigned_download_url
        );

        run_script(vec![&download_cmd], get_worker_dir())?;

        let unzip_cmd = format!("unzip {}.zip -d /root/{}", project_id, project_id);

        run_script(vec![&unzip_cmd], get_worker_dir())?;

        sleep(Duration::from_secs(3));

        let project_path = format!("/root/{}", project_id);

        let project_type = detect_project_type(&project_path);

        if !run_details.run_command.is_empty() {
            let run_cmd = format!("cd {} && {}", project_path, run_details.run_command);

            run_script_bg(vec![&run_cmd], get_worker_dir())?;

            return Ok(());
        }

        let config = get_default_config(project_type);
        let config_run_cmd = config.run_command.unwrap();

        let final_cmd = format!("cd {} && {}", project_path, config_run_cmd);

        run_script_bg(vec![&final_cmd], get_worker_dir())?;

        println!("Run command completed");
        Ok(())
    }

    pub async fn redeploy(&self, redeploy_details: &RedeployDetails) -> Result<(), AppError> {
        let project_id = redeploy_details.project_id.to_owned();

        let download_cmd = format!(
            "curl -o {}.zip '{}'",
            &project_id, redeploy_details.presigned_download_url
        );

        run_script(vec![&download_cmd], get_worker_dir())?;

        let unzip_cmd = format!("unzip {}.zip -d /root/{}", project_id, project_id);

        run_script(vec![&unzip_cmd], get_worker_dir())?;

        let copy_job_json = format!("cp /root/{}/job.json /root/", project_id);

        run_script(vec![&copy_job_json], get_worker_dir())?;

        let job_json_str = fs::read_to_string("/root/job.json")?;

        let mut job_json = serde_json::from_str::<DeployDetails>(&job_json_str)?;

        job_json.presigned_upload_url = redeploy_details.presigned_upload_url.to_owned();
        job_json.access_token = redeploy_details.access_token.to_owned();

        self.execute(&job_json).await?;

        Ok(())
    }
}
