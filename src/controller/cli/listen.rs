use std::{collections::HashMap, time::Duration};

use actix_web::{App, HttpResponse, HttpServer, web};
use futures::lock::Mutex;

use crate::{
    app_errors::AppError,
    app_types::{DeployDetails, DeployReq, EventType, InstallationEvent, RedeployDetails},
    controller::{
        api::github::Github,
        dispatcher::job_dispatcher::JobDispatcher,
        queue::{deploy_queue::DeployQueue, lapin::Lapin, redeploy_queue::ReDeployQueue},
        storage::s3::S3Service,
        vm::{firecracker::Firecracker, id_allocator::IdAllocator, vm_pool::VmPool},
    },
};

type InstallationStore = web::Data<Mutex<HashMap<String, InstallationEvent>>>;

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
) -> Result<HttpResponse, AppError> {
    println!("i was called");
    let deploy_details = serde_json::from_slice::<DeployReq>(&body).unwrap();

    println!("Deploy details: {:?}", deploy_details);

    deploy_queue.publish(&deploy_details).await?;
    Ok(HttpResponse::Ok().finish())
}

pub async fn listen(
    id_allocator: IdAllocator,
    vm_pool: VmPool,
    s3_service: S3Service,
) -> Result<(), AppError> {
    let installation_ids: InstallationStore =
        web::Data::new(Mutex::new(HashMap::<String, InstallationEvent>::new()));

    let job_dispatcher = JobDispatcher::new(vm_pool.clone(), s3_service.clone());

    for _ in 0..1 {
        let new_id = id_allocator.allocate_id().await? as u32;
        let mut new_vm = Firecracker::new(new_id);

        new_vm.create_vm().await?;
        vm_pool.add_to_ideal_vms(new_id);
    }

    let lapin_conn = Lapin::new().await?;
    let deploy_queue = web::Data::new(DeployQueue::new(&lapin_conn).await?);
    let redeploy_queue = web::Data::new(ReDeployQueue::new(&lapin_conn).await?);

    let s3_service = s3_service.clone();

    println!("Queue created");

    {
        let deploy_queue = deploy_queue.clone();
        let installation_ids = installation_ids.clone();
        let mut job_dispatcher = job_dispatcher.clone();
        let s3_service = s3_service.clone();

        tokio::spawn(async move {
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

                if let Err(e) = job_dispatcher.dispatch_deploy_job(&deploy_details).await {
                    eprintln!("Job dispatch failed: {:?}", e);
                }
            }
        });
    }

    {
        let redeploy_queue = redeploy_queue.clone();
        let mut job_dispatcher = job_dispatcher.clone();
        let s3_service = s3_service.clone();

        tokio::spawn(async move {
            loop {
                let redeploy_event = match redeploy_queue.consume().await {
                    Ok(ev) => ev,
                    Err(e) => {
                        eprintln!("Queue error: {:?}", e);
                        tokio::time::sleep(Duration::from_secs(1)).await;
                        continue;
                    }
                };

                let project_id = redeploy_event.repositories[0].full_name.replace("/", "-");

                let (owner, repo) = {
                    let parts: Vec<&str> = redeploy_event.repositories[0]
                        .full_name
                        .split('/')
                        .collect();
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

                if let Err(e) = job_dispatcher
                    .dispatch_redeploy_job(&redeploy_details)
                    .await
                {
                    eprintln!("Job dispatch failed: {:?}", e);
                }
            }
        });
    }

    HttpServer::new(move || {
        App::new()
            .app_data(installation_ids.clone())
            .app_data(deploy_queue.clone())
            .app_data(redeploy_queue.clone())
            .route("/webhook/github", web::post().to(github_webhook))
            .route("/deploy", web::post().to(deploy_handler))
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await?;

    Ok(())
}
