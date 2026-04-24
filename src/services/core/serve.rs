use std::time::Duration;

use crate::services::firecracker::vm_pool::VmPool;
use crate::utils::run_script::run_script;
use crate::{
    app_errors::AppError,
    services::firecracker::firecracker::Firecracker,
    utils::detect_project_type::{ProjectType, detect_project_type},
};
use actix_web::web;
use tokio::net::TcpStream;
use uuid::Uuid;

pub struct ServeCore;

impl ServeCore {
    pub fn new() -> Self {
        Self {}
    }

    async fn wait_for_port(vm_ip: u32, port: u16, max_wait: Duration) -> Result<(), AppError> {
        let addr = format!("172.16.0.{}:{}", vm_ip, port);
        let deadline = tokio::time::Instant::now() + max_wait;

        loop {
            if tokio::time::Instant::now() > deadline {
                return Err(AppError::StartingFirecrackerFailed(format!(
                    "Timed out waiting for port {}",
                    addr
                )));
            }
            match TcpStream::connect(&addr).await {
                Ok(_) => {
                    println!("Port ready: {}", addr);
                    return Ok(());
                }
                Err(_) => tokio::time::sleep(Duration::from_millis(200)).await,
            }
        }
    }

    pub async fn serve(
        &self,
        project_id: Uuid,
        vm_id: u32,
        run_script_vm: Vec<String>,
        vm_pool: web::Data<VmPool>,
    ) -> Result<(), AppError> {
        let project_type = detect_project_type(&project_id.to_string());

        match project_type {
            ProjectType::Unknown => {
                return Err(AppError::UnknownProjectType);
            }
            _ => {
                let mut new_vm = Firecracker::new(vm_id, ProjectType::Node);
                new_vm.create_vm().await?;
                vm_pool.add_to_pool(project_id, vm_id);

                let copy_dist_dir_to_microvm = format!(
                    "scp -r -i ubuntu.id_rsa /home/scrom/code/shipr/build/{} root@172.16.0.{}:/root/{}",
                    project_id,
                    vm_id + 2,
                    project_id
                );

                let run_script_final = run_script_vm.join(" && ");

                run_script(vec![&copy_dist_dir_to_microvm])?;
                new_vm.execute_command(&run_script_final).await?;

                // Wait until npx serve is actually listening before returning
                Self::wait_for_port(vm_id + 2, 3000, Duration::from_secs(30)).await?;
            }
        }
        Ok(())
    }
}
