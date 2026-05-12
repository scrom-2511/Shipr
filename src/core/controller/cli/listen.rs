use std::collections::HashMap;

use actix_web::{
    App, HttpServer,
    web::{self},
};
use tokio::sync::{Mutex, broadcast::Sender};

use crate::{
    app_errors::AppError,
    core::app_types::{InstallationEvent, InstallationStore, LogsStore},
    core::controller::{
        cli::{listen_deploy::listen_deploy, listen_redeploy::listen_redeploy},
        dispatcher::job_dispatcher::JobDispatcher,
        handlers::{
            deploy::deploy_handler,
            github::github_webhook,
            kill_vm::kill_vm_handler,
            logs::{logs_handler, stream_logs_handler},
            redeployment_completed::redeploy_completed_handler,
        },
        queue::{deploy_queue::DeployQueue, lapin::Lapin, redeploy_queue::ReDeployQueue},
        storage::s3::S3Service,
        vm::{
            firecracker::Firecracker, heartbeat_store::HeartbeatStore, id_allocator::IdAllocator,
            vm_pool::VmPool,
        },
    },
};

pub async fn listen(
    id_allocator: IdAllocator,
    vm_pool: VmPool,
    s3_service: S3Service,
    heartbeat_store: HeartbeatStore,
) -> Result<(), AppError> {
    let installation_ids: InstallationStore =
        web::Data::new(Mutex::new(HashMap::<String, InstallationEvent>::new()));

    let logs_store: LogsStore =
        web::Data::new(Mutex::new(HashMap::<String, Sender<String>>::new()));

    let job_dispatcher = JobDispatcher::new(
        vm_pool.clone(),
        s3_service.clone(),
        id_allocator.clone(),
        heartbeat_store.clone(),
    );

    for _ in 0..1 {
        let new_id = id_allocator.allocate_id().await?;

        println!("New ID listen: {}", new_id);

        let mut new_vm = Firecracker::new(new_id);

        new_vm.create_vm().await?;
        vm_pool.add_to_ideal_vms(new_id).await?;
    }

    let lapin_conn = Lapin::new().await?;
    let deploy_queue = web::Data::new(DeployQueue::new(&lapin_conn).await?);
    let redeploy_queue = web::Data::new(ReDeployQueue::new(&lapin_conn).await?);

    let s3_service = s3_service.clone();

    println!("Queue created");

    {
        let id_allocator = id_allocator.clone();
        let vm_pool = vm_pool.clone();
        let deploy_queue = deploy_queue.clone();
        let installation_ids = installation_ids.clone();
        let job_dispatcher = job_dispatcher.clone();
        let s3_service = s3_service.clone();

        tokio::spawn(async move {
            listen_deploy(
                installation_ids,
                s3_service,
                job_dispatcher,
                id_allocator,
                vm_pool,
                deploy_queue,
            )
            .await
        });
    }

    {
        let id_allocator = id_allocator.clone();
        let vm_pool = vm_pool.clone();
        let redeploy_queue = redeploy_queue.clone();
        let job_dispatcher = job_dispatcher.clone();
        let s3_service = s3_service.clone();

        tokio::spawn(async move {
            listen_redeploy(
                s3_service,
                job_dispatcher,
                id_allocator,
                vm_pool,
                redeploy_queue,
            )
            .await;
        });
    }

    let id_allocator = id_allocator.clone();
    let vm_pool = vm_pool.clone();

    let id_allocator = web::Data::new(id_allocator);
    let vm_pool = web::Data::new(vm_pool);

    HttpServer::new(move || {
        App::new()
            .app_data(installation_ids.clone())
            .app_data(deploy_queue.clone())
            .app_data(redeploy_queue.clone())
            .app_data(id_allocator.clone())
            .app_data(vm_pool.clone())
            .app_data(logs_store.clone())
            .route("/kill-vm", web::post().to(kill_vm_handler))
            .route("/webhook/github", web::post().to(github_webhook))
            .route("/deploy", web::post().to(deploy_handler))
            .route("/send-logs", web::post().to(logs_handler))
            .route("/logs/{project_id}", web::get().to(stream_logs_handler))
            .route(
                "/redeploy-completed",
                web::post().to(redeploy_completed_handler),
            )
    })
    .bind(("127.0.0.1", 3000))?
    .run()
    .await?;

    Ok(())
}
