use std::time::Duration;

use actix_web::web;

use crate::{
    app_errors::AppError,
    app_types::RedeployDetails,
    controller::{
        api::github::Github,
        dispatcher::job_dispatcher::JobDispatcher,
        queue::redeploy_queue::ReDeployQueue,
        storage::s3::S3Service,
        vm::{firecracker::Firecracker, id_allocator::IdAllocator, vm_pool::VmPool},
    },
};

pub async fn listen_redeploy(
    s3_service: S3Service,
    mut job_dispatcher: JobDispatcher,
    id_allocator: IdAllocator,
    vm_pool: VmPool,
    redeploy_queue: web::Data<ReDeployQueue>,
) {
    loop {
        let redeploy_event = match redeploy_queue.consume().await {
            Ok(ev) => ev,
            Err(e) => {
                eprintln!("Queue error: {:?}", e);
                tokio::time::sleep(Duration::from_secs(1)).await;
                continue;
            }
        };

        let project_id = redeploy_event.repository.full_name.replace("/", "-");

        let (owner, repo) = {
            let parts: Vec<&str> = redeploy_event.repository.full_name.split('/').collect();
            (parts[0].to_string(), parts[1].to_string())
        };

        let github = Github::new(3566236, &owner, &repo);

        let presigned_upload_url = s3_service
            .get_presigned_upload_url(&project_id)
            .await
            .unwrap();

        let presigned_download_url = s3_service
            .get_presigned_download_url(&project_id)
            .await
            .unwrap();

        let access_token = github.get_installation_access_token().await.unwrap();

        let redeploy_details = RedeployDetails {
            commit_hash: redeploy_event.after,
            presigned_download_url,
            presigned_upload_url,
            project_id,
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

        if let Err(e) = job_dispatcher
            .dispatch_redeploy_job(&redeploy_details)
            .await
        {
            eprintln!("Job dispatch failed: {:?}", e);
        }
    }
}
