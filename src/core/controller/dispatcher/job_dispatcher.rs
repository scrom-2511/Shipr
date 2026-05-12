use core::fmt;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use url::Url;

use crate::app_errors::AppError;
use crate::core::app_types::{DeployDetails, JobType, RedeployDetails, RunDetails};
use crate::core::config::app_config::get_dir;
use crate::core::controller::storage::s3::S3Service;
use crate::core::controller::vm::firecracker::Firecracker;
use crate::core::controller::vm::heartbeat_store::HeartbeatStore;
use crate::core::controller::vm::id_allocator::IdAllocator;
use crate::core::controller::vm::vm_pool::VmPool;
use crate::core::infra::kill_vm::kill_vm;
use crate::core::infra::process::run_script;

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
    id_allocator: IdAllocator,
    heartbeat_store: HeartbeatStore,
}

impl JobDispatcher {
    pub fn new(
        vm_pool: VmPool,
        s3_service: S3Service,
        id_allocator: IdAllocator,
        heartbeat_store: HeartbeatStore,
    ) -> Self {
        Self {
            vm: None,
            vm_pool,
            s3_service,
            id_allocator,
            heartbeat_store,
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

    async fn get_or_create_vm(
        &mut self,
        project_id: &str,
        job_type: JobType,
    ) -> Result<(u8, bool), AppError> {
        let something = self.vm_pool.get_from_pool(project_id, &job_type).await?;

        match something {
            Some(id) => {
                self.vm = Some(Firecracker::new(id));

                println!("VM found in pool: {}", id);
                Ok((id, false))
            }
            None => {
                let new_id = self
                    .vm_pool
                    .get_from_ideal_vms()
                    .await
                    .map_err(|_| AppError::NoAvailableVm)?
                    .unwrap();

                self.vm_pool
                    .add_to_pool(project_id, &job_type, new_id)
                    .await?;
                self.vm = Some(Firecracker::new(new_id));

                println!("VM from ideal: {}", new_id);

                Ok((new_id, true))
            }
        }
    }

    async fn move_json_to_vm(&self, vm_details: &impl VmDetails) -> Result<(), AppError> {
        let vm = self.vm.as_ref().expect("VM not found");
        vm.get_base_id();

        println!("vm base_id is: {}", vm.get_base_id());

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

        let (vm_id, _) = self
            .get_or_create_vm(&deploy_details.project_id, JobType::Deploy)
            .await?;

        self.move_json_to_vm(deploy_details).await?;

        let base_id = vm_id * 4;

        let vm_ip = format!("172.16.0.{}", base_id + 2);

        run_script(
            vec![&format!(
                "scp -r -i /home/scrom/ubuntu.id_rsa /home/scrom/code/shipr/target/release/worker root@{}:/root/worker",
                vm_ip
            )],
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
        let (vm_id, _) = self
            .get_or_create_vm(&redeploy_details.project_id, JobType::Redeploy)
            .await?;

        self.move_json_to_vm(redeploy_details).await?;

        let base_id = vm_id * 4;

        let vm_ip = format!("172.16.0.{}", base_id + 2);

        run_script(
            vec![&format!(
                "scp -r -i /home/scrom/ubuntu.id_rsa /home/scrom/code/shipr/target/release/worker root@{}:/root/worker",
                vm_ip
            )],
            get_dir(),
        )?;

        self.vm
            .as_ref()
            .unwrap()
            .execute_command("cd /root && ./worker job.json redeploy")?;

        Ok(())
    }

    pub async fn dispatch_run_job(&mut self, project_id: &str) -> Result<(), AppError> {
        let (vm_id, is_new) = self.get_or_create_vm(&project_id, JobType::Run).await?;

        println!("is new is: {}", is_new);

        let base_id = vm_id * 4;

        let vm_ip = format!("172.16.0.{}", base_id + 2);

        run_script(
            vec![&format!(
                "scp -r -i /home/scrom/ubuntu.id_rsa /home/scrom/code/shipr/target/release/worker root@{}:/root/worker",
                vm_ip
            )],
            get_dir(),
        )?;

        println!("cp cmd run completed");

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

            let id_allocator = self.id_allocator.clone();
            let vm_pool = self.vm_pool.clone();
            let heartbeat_store = self.heartbeat_store.clone();
            let project_id = project_id.to_string();

            tokio::task::spawn(async move {
                let new_id = id_allocator.allocate_id().await?;
                let mut new_vm = Firecracker::new(new_id);

                new_vm.create_vm().await?;
                vm_pool.add_to_ideal_vms(new_id).await?;

                let mut count = 0;

                loop {
                    tokio::time::sleep(Duration::from_secs(1)).await;

                    count += 1;

                    println!("count is: {}", count);

                    let dead = heartbeat_store.is_dead(&project_id).await?;

                    if dead {
                        kill_vm(&project_id, &JobType::Run, &vm_pool, &id_allocator).await?;
                        break;
                    }
                }

                Ok::<(), AppError>(())
            });

            self.vm
                .as_ref()
                .unwrap()
                .execute_command_bg("cd /root && ./worker job.json run")?;
        }

        Ok(())
    }
}
