use std::time::Duration;

use actix_web::web;

use crate::{
    app_errors::AppError,
    core::{
        app_types::DeployDetails,
        controller::{
            dispatcher::job_dispatcher::JobDispatcher,
            queue::deploy_queue::DeployQueue,
            storage::s3::S3Service,
            vm::{firecracker::Firecracker, id_allocator::IdAllocator, vm_pool::VmPool},
        },
    },
    shared::github_app::GithubApp,
};

pub async fn listen_deploy(
    s3_service: S3Service,
    mut job_dispatcher: JobDispatcher,
    id_allocator: IdAllocator,
    vm_pool: VmPool,
    deploy_queue: web::Data<DeployQueue>,
) {
    loop {
        let deploy_details_req = match deploy_queue.pop_from_queue().await {
            Ok(v) => v,
            Err(e) => {
                eprintln!("Queue error: {:?}", e);
                tokio::time::sleep(Duration::from_secs(1)).await;
                continue;
            }
        };

        let github = GithubApp::new();

        let access_token = github.get_installation_access_token(12345).await.unwrap();

        let url = deploy_details_req.url.clone();

        let cleaned_url = url.replace(".git", "");

        println!("Cleaned URL: {}", cleaned_url);

        let full_name = deploy_details_req.full_name;

        let project_id = full_name.replace("/", "-");

        let (owner, repo) = {
            let parts: Vec<&str> = full_name.split('/').collect();
            (parts[0].to_string(), parts[1].to_string())
        };

        let presigned_upload_url = s3_service
            .get_presigned_upload_url(&project_id)
            .await
            .unwrap();

        println!("Access Token fetched");

        let deploy_details = DeployDetails {
            url: cleaned_url,
            install_commands: deploy_details_req.install,
            build_commands: deploy_details_req.build,
            branch: deploy_details_req.branch,
            project_id: project_id.to_string(),
            home_dir: deploy_details_req.home_dir,
            dist_dir: deploy_details_req.dist_dir,
            presigned_upload_url,
            owner: owner.to_string(),
            repo: repo.to_string(),
            access_token,
        };

        let id_allocator = id_allocator.clone();
        let vm_pool = vm_pool.clone();

        tokio::task::spawn(async move {
            let mut new_vm = Firecracker::new_from_id_allocator(&id_allocator).await;
            new_vm.create_new_vm_and_add_to_pool(&vm_pool).await?;

            Ok::<(), AppError>(())
        });

        if let Err(e) = job_dispatcher.dispatch_deploy_job(&deploy_details).await {
            eprintln!("Job dispatch failed: {:?}", e);
        }
    }
}
