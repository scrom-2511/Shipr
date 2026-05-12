use std::{collections::HashMap, fs};

use actix_web::{
    App, HttpResponse, HttpServer,
    web::{self},
};
use serde_json::Value;
use tokio::sync::{
    Mutex,
    broadcast::{Sender, channel},
};

use crate::{
    app_errors::AppError,
    app_types::{
        DeployReq, EventType, InstallationEvent, InstallationStore, JobType, KillVmReq, LogsStore,
    },
    controller::{
        api::logs::{logs_handler, stream_logs_handler},
        cli::{listen_deploy::listen_deploy, listen_redeploy::listen_redeploy},
        dispatcher::job_dispatcher::JobDispatcher,
        queue::{deploy_queue::DeployQueue, lapin::Lapin, redeploy_queue::ReDeployQueue},
        storage::s3::S3Service,
        vm::{
            firecracker::Firecracker, heartbeat_store::HeartbeatStore, id_allocator::IdAllocator,
            vm_pool::VmPool,
        },
    },
    infra::kill_vm::kill_vm,
};

async fn redeploy_completed_handler(
    body: web::Bytes,
    vm_pool: web::Data<VmPool>,
) -> Result<HttpResponse, AppError> {
    let redeploy_details = serde_json::from_slice::<Value>(&body).unwrap();

    let project_id = redeploy_details["project_id"].as_str().unwrap();

    vm_pool
        .remove_from_pool(project_id, &JobType::Redeploy)
        .await?;

    Ok(HttpResponse::Ok().finish())
}

async fn kill_vm_handler(
    body: web::Bytes,
    vm_pool: web::Data<VmPool>,
    id_allocator: web::Data<IdAllocator>,
    logs_store: LogsStore,
) -> Result<HttpResponse, AppError> {
    let kill_vm_req = serde_json::from_slice::<KillVmReq>(&body).unwrap();

    println!("Kill VM request: {:?}", kill_vm_req);

    kill_vm(
        &kill_vm_req.project_id,
        &kill_vm_req.job_type,
        &vm_pool,
        &id_allocator,
    )
    .await?;

    logs_store.lock().await.remove(&kill_vm_req.project_id);

    Ok(HttpResponse::Ok().finish())
}

async fn github_webhook(
    body: web::Bytes,
    installation_ids: InstallationStore,
    redeploy_queue: web::Data<ReDeployQueue>,
) -> HttpResponse {
    println!("Github webhook received");
    println!(
        "Github webhook received: {}",
        String::from_utf8_lossy(&body)
    );

    let event = serde_json::from_slice::<EventType>(&body).unwrap();

    match event {
        EventType::Install(installation_event) => {
            if installation_event.action == "created" {
                let url = format!(
                    "https://github.com/{}",
                    installation_event.repositories[0].full_name
                );

                println!("the key url is: {}", url);

                installation_ids
                    .lock()
                    .await
                    .insert(url.clone(), installation_event);

                println!("Installation event stored");

                println!(
                    "installation_ids from fn: {:?}",
                    installation_ids.lock().await
                );
            }
        }
        EventType::Push(redeploy_event) => {
            redeploy_queue.publish(redeploy_event).await.unwrap();

            println!("Installation event stored");

            println!(
                "installation_ids from fn: {:?}",
                installation_ids.lock().await
            );
        }
    }

    HttpResponse::Ok().finish()
}

async fn deploy_handler(
    body: web::Bytes,
    deploy_queue: web::Data<DeployQueue>,
    logs_store: LogsStore,
) -> Result<HttpResponse, AppError> {
    println!("i was called");
    let deploy_details = serde_json::from_slice::<DeployReq>(&body).unwrap();

    println!("Deploy details: {:?}", deploy_details);

    let mut url = deploy_details.url.trim().to_string();

    url = url.replace("https://github.com/", "");
    url = url.replace(".git", "");

    if url.ends_with('/') {
        url.pop();
    }

    url = url.replace("/", "-");

    println!("{}", url);

    let project_id = url;

    let (tx, _) = channel::<String>(100);

    let file_path = "/home/scrom/code/shipr/logs";

    fs::create_dir_all(file_path).unwrap();

    fs::File::create(format!("{}/{}.txt", file_path, project_id)).unwrap();

    // let log = "Waiting for queue...";

    // let mut file = fs::OpenOptions::new()
    //     .create(true)
    //     .append(true)
    //     .open(format!("{}/{}.txt", file_path, project_id))
    //     .unwrap();

    // writeln!(file, "{}", log).unwrap();

    logs_store.lock().await.insert(project_id.clone(), tx);

    println!("{:?}", logs_store.lock().await.keys());

    deploy_queue.publish(&deploy_details).await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "project_id": project_id
    })))
}

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
