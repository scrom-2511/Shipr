use core::fmt;
use std::fs;
use std::path::PathBuf;
use url::Url;

use crate::app_errors::AppError;
use crate::app_types::{DeployDetails, JobType, RedeployDetails, RunDetails};
use crate::config::app_config::get_dir;
use crate::controller::storage::s3::S3Service;
use crate::controller::vm::firecracker::Firecracker;
use crate::controller::vm::vm_pool::VmPool;
use crate::infra::process::run_script;

impl fmt::Display for JobType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JobType::Deploy => write!(f, "deploy"),
            JobType::Run => write!(f, "run"),
            JobType::Redeploy => write!(f, "redeploy"),
        }
    }
}

pub trait VmDetails {
    fn get_json(&self) -> String;
    fn get_project_id(&self) -> String;
    fn get_job_type(&self) -> JobType;
}

impl VmDetails for DeployDetails {
    fn get_json(&self) -> String {
        serde_json::to_string(self).unwrap()
    }

    fn get_project_id(&self) -> String {
        self.project_id.to_string()
    }

    fn get_job_type(&self) -> JobType {
        JobType::Deploy
    }
}

impl VmDetails for RunDetails {
    fn get_json(&self) -> String {
        serde_json::to_string(self).unwrap()
    }

    fn get_project_id(&self) -> String {
        self.project_id.to_string()
    }

    fn get_job_type(&self) -> JobType {
        JobType::Run
    }
}

impl VmDetails for RedeployDetails {
    fn get_json(&self) -> String {
        serde_json::to_string(self).unwrap()
    }

    fn get_project_id(&self) -> String {
        self.project_id.to_string()
    }

    fn get_job_type(&self) -> JobType {
        JobType::Redeploy
    }
}

#[derive(Clone)]
pub struct JobDispatcher {
    vm: Option<Firecracker>,
    vm_pool: VmPool,
    pub s3_service: S3Service,
}

impl JobDispatcher {
    pub fn new(vm_pool: VmPool, s3_service: S3Service) -> Self {
        Self {
            vm: None,
            vm_pool,
            s3_service,
        }
    }

    fn git_url_validator(&self, git_url: &str) -> Result<bool, AppError> {
        if let Ok(url) = Url::parse(git_url) {
            if url.host_str() == Some("github.com") && url.scheme() == "https" {
                return Ok(true);
            }
        }

        Err(AppError::InvalidGitUrl)
    }

    async fn get_or_create_vm(&mut self, project_id: &str) -> Result<(u8, bool), AppError> {
        let something = self.vm_pool.get_from_pool(&project_id).await?;

        println!("{:?}", something);

        match something {
            Some(id) => {
                self.vm = Some(Firecracker::new(id));
                Ok((id, false))
            }
            None => {
                let new_id = self
                    .vm_pool
                    .get_from_ideal_vms()
                    .await
                    .map_err(|_| AppError::NoAvailableVm)?;

                self.vm_pool.add_to_pool(&project_id, new_id).await?;
                self.vm = Some(Firecracker::new(new_id));

                Ok((new_id, true))
            }
        }
    }

    async fn move_json_to_vm(&self, vm_details: &impl VmDetails) -> Result<(), AppError> {
        let vm = self.vm.as_ref().expect("VM not found");
        vm.get_base_id();

        let job_json = vm_details.get_json();

        let project_id = vm_details.get_project_id();

        let job_type = vm_details.get_job_type().to_string().to_lowercase();

        let job_json_path = format!(
            "/home/scrom/code/shipr/job_jsons/{}/{}.json",
            job_type, project_id
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

        run_script(vec![&copy_job_json_to_vm], get_dir())?;

        Ok(())
    }

    pub async fn dispatch_deploy_job(
        &mut self,
        deploy_details: &DeployDetails,
    ) -> Result<(), AppError> {
        self.git_url_validator(&deploy_details.url)?;
        self.get_or_create_vm(&deploy_details.project_id).await?;
        self.move_json_to_vm(deploy_details).await?;

        run_script(
            vec![
                "scp -r -i /home/scrom/ubuntu.id_rsa /home/scrom/code/shipr/target/release/worker root@172.16.0.2:/root/worker",
            ],
            get_dir(),
        )?;

        self.vm
            .as_ref()
            .unwrap()
            .execute_command("cd /root && ./worker job.json deploy")?;

        Ok(())
    }

    pub async fn dispatch_redeploy_job(
        &mut self,
        redeploy_details: &RedeployDetails,
    ) -> Result<(), AppError> {
        self.get_or_create_vm(&redeploy_details.project_id).await?;
        self.move_json_to_vm(redeploy_details).await?;

        run_script(
            vec![
                "scp -r -i /home/scrom/ubuntu.id_rsa /home/scrom/code/shipr/target/release/worker root@172.16.0.2:/root/worker",
            ],
            get_dir(),
        )?;

        self.vm
            .as_ref()
            .unwrap()
            .execute_command("cd /root && ./worker job.json redeploy")?;

        Ok(())
    }

    pub async fn dispatch_run_job(&mut self, project_id: &str) -> Result<(), AppError> {
        let (_, is_new) = self.get_or_create_vm(&project_id).await?;

        run_script(
            vec![
                "scp -r -i /home/scrom/ubuntu.id_rsa /home/scrom/code/shipr/target/release/worker root@172.16.0.2:/root/worker",
            ],
            get_dir(),
        )?;

        if is_new == true {
            let presigned_download_url = self
                .s3_service
                .get_presigned_download_url(&project_id.to_string())
                .await?;

            let run_details = RunDetails {
                presigned_download_url,
                run_command: "".to_string(),
                project_id: project_id.to_string(),
            };

            self.move_json_to_vm(&run_details).await?;

            // self.vm
            //     .as_ref()
            //     .unwrap()
            //     .execute_command("cd /root && nohup ./worker job.json run > /dev/null 2>&1 &")?;

            self.vm
                .as_ref()
                .unwrap()
                .execute_command_bg("cd /root && ./worker job.json run")?;
        }

        Ok(())
    }
}
