use core::fmt;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use url::Url;
use uuid::Uuid;

use crate::app_errors::AppError;
use crate::app_types::{DeployDetails, JobType, RunDetails};
use crate::controller::storage::s3::S3Service;
use crate::controller::vm::firecracker::Firecracker;
use crate::controller::vm::vm_pool::VmPool;
use crate::infra::process::{run_script, run_script_vm_bg};

impl fmt::Display for JobType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JobType::Deploy => write!(f, "deploy"),
            JobType::Run => write!(f, "run"),
        }
    }
}

pub trait VmDetails {
    fn get_json(&self) -> String;
    fn get_project_id(&self) -> Uuid;
    fn get_job_type(&self) -> JobType;
}

impl VmDetails for DeployDetails {
    fn get_json(&self) -> String {
        serde_json::to_string(self).unwrap()
    }

    fn get_project_id(&self) -> Uuid {
        self.project_id
    }

    fn get_job_type(&self) -> JobType {
        JobType::Deploy
    }
}

impl VmDetails for RunDetails {
    fn get_json(&self) -> String {
        serde_json::to_string(self).unwrap()
    }

    fn get_project_id(&self) -> Uuid {
        self.project_id
    }

    fn get_job_type(&self) -> JobType {
        JobType::Run
    }
}

#[derive(Clone)]
pub struct JobDispatcher {
    vm: Option<Firecracker>,
    vm_pool: VmPool,
    s3_service: S3Service,
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

    async fn get_or_create_vm(&mut self, project_id: Uuid) -> Result<(u32, bool), AppError> {
        match self.vm_pool.get_from_pool(project_id) {
            Some(id) => {
                println!("Using existing VM {}", id);
                self.vm = Some(Firecracker::new(id));
                Ok((id, false))
            }
            None => {
                let new_id = self
                    .vm_pool
                    .get_from_ideal_vms()
                    .ok_or(AppError::NoAvailableVm)?;

                self.vm_pool.add_to_pool(project_id, new_id);
                self.vm = Some(Firecracker::new(new_id));

                println!("Starting VM {}", new_id);

                Ok((new_id as u32, true))
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
            "/home/scrom/shipr/job_jsons/{}/{}.json",
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

        run_script(vec![&copy_job_json_to_vm])?;

        Ok(())
    }

    pub async fn dispatch_deploy_job(
        &mut self,
        deploy_details: &DeployDetails,
    ) -> Result<(), AppError> {
        self.git_url_validator(&deploy_details.url)?;
        self.get_or_create_vm(deploy_details.project_id).await?;
        self.move_json_to_vm(deploy_details).await?;

        run_script(vec![
            "scp -r -i /home/scrom/ubuntu.id_rsa /home/scrom/code/shipr/target/release/worker root@172.16.0.2:/root/worker",
        ])?;

        self.vm
            .as_ref()
            .unwrap()
            .execute_command("cd /root && ./worker job.json deploy")?;

        Ok(())
    }

    pub async fn dispatch_run_job(&mut self, project_id: Uuid) -> Result<(), AppError> {
        println!("Dispatching run job for project {}", project_id);
        let (vm_id, is_new) = self.get_or_create_vm(project_id).await?;

        println!(
            ">>> dispatch_run_job: project={} vm_id={} is_new={}",
            project_id, vm_id, is_new
        );
        println!(
            ">>> pool state: {:?}",
            self.vm_pool.get_from_pool(project_id)
        );

        println!("VM is new: {}", is_new);

        run_script(vec![
            "scp -r -i /home/scrom/ubuntu.id_rsa /home/scrom/code/shipr/target/release/worker root@172.16.0.2:/root/worker",
        ])?;

        if is_new == true {
            println!("Getting presigned download URL");
            let presigned_download_url = self
                .s3_service
                .get_presigned_download_url(&project_id.to_string())
                .await?;

            let run_details = RunDetails {
                presigned_download_url,
                run_command: "".to_string(),
                project_id,
            };

            println!("Moving JSON to VM");

            self.move_json_to_vm(&run_details).await?;

            println!("Copying worker to VM");

            println!("Executing command in VM");

            // self.vm
            //     .as_ref()
            //     .unwrap()
            //     .execute_command("cd /root && nohup ./worker job.json run > /dev/null 2>&1 &")?;

            run_script_vm_bg(vec!["cd /root && ./worker job.json run"])?;
        }

        println!(">>> execute_command returned, is_new was: {}", is_new);

        Ok(())
    }
}
