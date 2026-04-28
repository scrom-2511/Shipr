use std::fs;
use std::path::PathBuf;

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

    async fn move_json_to_vm(&self, deploy_details: &DeployDetails) -> Result<(), AppError> {
        let vm = self.vm.as_ref().unwrap();
        vm.get_base_id();

        let job_json = serde_json::to_string(deploy_details)?;

        let job_json_path = format!(
            "/home/scrom/shipr/job_jsons/{}.json",
            deploy_details.unique_id
        );

        if let Some(parent) = PathBuf::from(&job_json_path).parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(&job_json_path, job_json)?;

        let copy_job_json_to_vm = format!(
            "scp -r -i ubuntu.id_rsa {} root@172.16.0.{}:/root/job.json",
            job_json_path,
            vm.get_base_id() + 2
        );

        run_script(vec![&copy_job_json_to_vm])?;

        Ok(())
    }

    pub async fn pull_build_setup(
        &mut self,
        deploy_details: &DeployDetails,
        id_allocator: UniqueIdAllocator,
    ) -> Result<(), AppError> {
        self.git_url_validator(&deploy_details.url)?;
        self.create_vm(id_allocator).await?;
        self.move_json_to_vm(deploy_details).await?;

        run_script(vec![
            "scp -r -i /home/scrom/ubuntu.id_rsa /home/scrom/code/shipr/target/release/worker root@172.16.0.2:/root/worker",
        ])?;

        self.vm
            .as_ref()
            .unwrap()
            .execute_command("cd /root && ./worker job.json")?;

        Ok(())
    }
}
