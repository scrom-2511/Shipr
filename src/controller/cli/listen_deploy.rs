use std::time::Duration;

use actix_web::web;

use crate::{
    app_errors::AppError,
    app_types::{DeployDetails, InstallationStore},
    controller::{
        api::github::Github,
        dispatcher::job_dispatcher::JobDispatcher,
        queue::deploy_queue::DeployQueue,
        storage::s3::S3Service,
        vm::{firecracker::Firecracker, id_allocator::IdAllocator, vm_pool::VmPool},
    },
};

pub async fn listen_deploy(
    installation_ids: InstallationStore,
    s3_service: S3Service,
    mut job_dispatcher: JobDispatcher,
    id_allocator: IdAllocator,
    vm_pool: VmPool,
    deploy_queue: web::Data<DeployQueue>,
) {
    loop {
        let deploy_details_req = match deploy_queue.consume().await {
            Ok(v) => v,
            Err(e) => {
                eprintln!("Queue error: {:?}", e);
                tokio::time::sleep(Duration::from_secs(1)).await;
                continue;
            }
        };

        let url = deploy_details_req.url.clone();
        println!("Received job for URL: {}", url);

        let installation_event = loop {
            if let Some(ev) = installation_ids.lock().await.get(&url).cloned() {
                break ev;
            }

            println!("waiting...");
            println!("installation_ids: {:?}", installation_ids.lock().await);

            tokio::time::sleep(Duration::from_secs(1)).await;
        };

        let cleaned_url = url.replace(".git", "");

        println!("Cleaned URL: {}", cleaned_url);

        let project_id = installation_event.repositories[0]
            .full_name
            .replace("/", "-");

        let (owner, repo) = {
            let parts: Vec<&str> = installation_event.repositories[0]
                .full_name
                .split('/')
                .collect();
            (parts[0].to_string(), parts[1].to_string())
        };

        let presigned_upload_url = s3_service
            .get_presigned_upload_url(&project_id)
            .await
            .unwrap();

        let github = Github::new(3566236, &owner, &repo);

        let access_token = github.get_installation_access_token().await.unwrap();

        println!("Access Token fetched");

        let deploy_details = DeployDetails {
            url: cleaned_url,
            install_commands: deploy_details_req.install,
            build_commands: deploy_details_req.build,
            branch: deploy_details_req.branch,
            project_id,
            home_dir: deploy_details_req.home_dir,
            dist_dir: deploy_details_req.dist_dir,
            presigned_upload_url,
            owner,
            repo,
            access_token,
        };

        let id_allocator = id_allocator.clone();
        let vm_pool = vm_pool.clone();

        tokio::task::spawn(async move {
            let new_id = id_allocator.allocate_id().await?;
            let mut new_vm = Firecracker::new(new_id);

            new_vm.create_vm().await?;
            vm_pool.add_to_ideal_vms(new_id).await?;

            Ok::<(), AppError>(())
        });

        if let Err(e) = job_dispatcher.dispatch_deploy_job(&deploy_details).await {
            eprintln!("Job dispatch failed: {:?}", e);
        }
    }
}
