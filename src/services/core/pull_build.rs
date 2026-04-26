use crate::services::firecracker::unique_id_allocator::UniqueIdAllocator;
use crate::utils::detect_project_type::ProjectType;
use crate::utils::run_script::run_script;
use crate::{
    app_errors::AppError, app_types::DeployDetails, services::firecracker::firecracker::Firecracker,
};
use url::Url;

pub struct PullBuildCore {
    vm: Option<Firecracker>,
}

impl PullBuildCore {
    pub fn new() -> Self {
        Self { vm: None }
    }

    fn git_url_validator(&self, git_url: &str) -> Result<bool, AppError> {
        if let Ok(url) = Url::parse(git_url) {
            if url.host_str() == Some("github.com") && url.scheme() == "https" {
                return Ok(true);
            }
        }

        Err(AppError::InvalidGitUrl)
    }

    async fn create_vm(&mut self, id_allocator: UniqueIdAllocator) -> Result<(), AppError> {
        let vm_id = id_allocator.allocate_id().await?;
        let mut new_vm = Firecracker::new(vm_id as u32, ProjectType::Node);
        new_vm.create_vm().await?;
        self.vm = Some(new_vm);

        Ok(())
    }

    fn extract_repo_name(&self, url: &str) -> Option<String> {
        let url = url.trim();

        let last_part = url.split('/').last()?;

        let repo_name = last_part.strip_suffix(".git").unwrap_or(last_part);

        Some(repo_name.to_string())
    }

    async fn pull(&self, deploy_details: &DeployDetails) -> Result<(), AppError> {
        let git_url = &deploy_details.url;

        self.git_url_validator(git_url)?;

        let repo_name = self
            .extract_repo_name(git_url)
            .ok_or(AppError::InvalidGitUrl)?;

        let git_clone_cmd = format!("git clone {}", git_url);

        let install_cmd = deploy_details.install_commands.join(" && ");

        let final_install_cmd = format!(
            "cd {} && cd {} && {}",
            repo_name, deploy_details.home_dir, install_cmd
        );

        let vm = self.vm.as_ref().unwrap();

        vm.execute_command(&git_clone_cmd)?;
        vm.execute_command(&final_install_cmd)?;

        Ok(())
    }

    async fn build(&self, deploy_details: &DeployDetails) -> Result<(), AppError> {
        let repo_name = self
            .extract_repo_name(&deploy_details.url)
            .ok_or(AppError::InvalidGitUrl)?;

        let build_cmd = deploy_details.build_commands.join(" && ");

        let final_build_cmd = format!(
            "cd {} && cd {} && {}",
            repo_name, deploy_details.home_dir, build_cmd
        );

        let vm = self.vm.as_ref().unwrap();

        vm.execute_command(&final_build_cmd)?;

        let mkdir_build_dir = format!(
            "mkdir -p /home/scrom/code/shipr/src/build/{}",
            deploy_details.unique_id
        );

        let vm_id = vm.get_base_id();

        let copy_dist_dir_to_host = format!(
            "scp -r -i ubuntu.id_rsa root@172.16.0.{}:./{}/{} /home/scrom/code/shipr/src/build/{}",
            vm_id + 2,
            repo_name,
            deploy_details.dist_dir,
            deploy_details.unique_id
        );

        run_script(vec![&mkdir_build_dir, &copy_dist_dir_to_host])?;

        vm.destroy_vm().await?;

        Ok(())
    }

    pub async fn pull_build(
        &mut self,
        deploy_details: &DeployDetails,
        id_allocator: UniqueIdAllocator,
    ) -> Result<(), AppError> {
        self.create_vm(id_allocator).await?;
        self.pull(deploy_details).await?;
        self.build(deploy_details).await?;

        Ok(())
    }
}
