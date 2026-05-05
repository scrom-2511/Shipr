use std::{collections::HashMap, sync::Arc, time::Duration};

use actix_web::{
    App, HttpRequest, HttpResponse, HttpServer,
    web::{self},
};
use clap::{Parser, Subcommand};
use futures::lock::Mutex;
use reqwest::Client;
use tokio::task;

use crate::{
    app_errors::AppError,
    app_types::{DeployDetails, DeployReq, EventType, InstallationEvent},
    config::app_config::get_dir,
    controller::{
        api::{github::Github, vm_request_proxy::VmRequestProxy},
        dispatcher::job_dispatcher::JobDispatcher,
        queue::{lapin::Lapin, pull_queue::PullQueue},
        storage::s3::S3Service,
        vm::{firecracker::Firecracker, id_allocator::IdAllocator, vm_pool::VmPool},
    },
    infra::process::run_script,
};

pub async fn proxy(
    vm_request_proxy: web::Data<Mutex<VmRequestProxy>>,
    req: HttpRequest,
    body: web::Bytes,
) -> Result<HttpResponse, AppError> {
    vm_request_proxy.lock().await.proxy_request(req, body).await
}

type InstallationStore = web::Data<Mutex<HashMap<String, InstallationEvent>>>;

async fn github_webhook(body: web::Bytes, installation_ids: InstallationStore) -> HttpResponse {
    println!("Github webhook received");
    println!(
        "Github webhook received: {}",
        String::from_utf8_lossy(&body)
    );

    let event = serde_json::from_slice::<EventType>(&body).unwrap();

    match event {
        EventType::Install(installation_event) => {
            println!("Installation event");
            if installation_event.action == "created" {
                println!(
                    "Installation created: {:?}",
                    installation_event.installation.id
                );
            };

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
        EventType::Push(push_event) => {
            println!("Push event");
            // let github = Github::new("3566236".to_string());

            // let token = github
            //     .get_installation_access_token(&installation_event.installation.id)
            //     .await?;

            // let url = format!(
            //     "https://x-access-token:{}@github.com/{}.git",
            //     token, installation_event.repositories.full_name
            // );
        }
    }

    HttpResponse::Ok().finish()
}

async fn deploy_handler(
    body: web::Bytes,
    queue: web::Data<PullQueue>,
) -> Result<HttpResponse, AppError> {
    println!("i was called");
    let deploy_details = serde_json::from_slice::<DeployReq>(&body).unwrap();

    println!("Deploy details: {:?}", deploy_details);

    queue.publish(&deploy_details).await?;
    Ok(HttpResponse::Ok().finish())
}
#[derive(Parser)]
#[command(name = "shipr")]
#[command(about = "Shipr CLI", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Deploy {
        #[arg(long)]
        url: String,

        #[arg(long)]
        install: Vec<String>,

        #[arg(long)]
        build: Vec<String>,

        #[arg(long)]
        branch: String,

        #[arg(long)]
        home_dir: String,

        #[arg(long)]
        dist_dir: String,
    },
    Serve,
    Listen,
    Test,
}

pub async fn cli(
    vm_pool: VmPool,
    id_allocator: IdAllocator,
    s3_service: S3Service,
) -> Result<(), AppError> {
    let args = Cli::parse();

    let installation_ids: InstallationStore =
        web::Data::new(Mutex::new(HashMap::<String, InstallationEvent>::new()));

    let job_dispatcher = JobDispatcher::new(vm_pool.clone(), s3_service.clone());

    match args.command {
        Commands::Listen {} => {
            println!("Starting listener...");

            for _ in 0..1 {
                let new_id = id_allocator.allocate_id().await? as u32;
                let mut new_vm = Firecracker::new(new_id);

                new_vm.create_vm().await?;
                vm_pool.add_to_ideal_vms(new_id);
            }

            let lapin_conn = Lapin::new().await?;
            let queue = web::Data::new(PullQueue::new(lapin_conn).await?);

            let s3_service = s3_service.clone();

            println!("Queue created");

            {
                let queue = queue.clone();
                let installation_ids = installation_ids.clone();
                let mut job_dispatcher = job_dispatcher.clone();
                let s3_service = s3_service.clone();

                tokio::spawn(async move {
                    loop {
                        let deploy_details_req = match queue.consume().await {
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

                        let presigned_upload_url =
                            match s3_service.get_presigned_upload_url(&project_id).await {
                                Ok(v) => v,
                                Err(e) => {
                                    eprintln!("S3 error: {:?}", e);
                                    continue;
                                }
                            };

                        let (owner, repo) = {
                            let parts: Vec<&str> = installation_event.repositories[0]
                                .full_name
                                .split('/')
                                .collect();
                            (parts[0].to_string(), parts[1].to_string())
                        };

                        let github = Github::new(3566236, &owner, &repo);

                        let access_token = match github.get_installation_access_token().await {
                            Ok(v) => v,
                            Err(e) => {
                                eprintln!("Github token error: {:?}", e);
                                continue;
                            }
                        };

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

            HttpServer::new(move || {
                App::new()
                    .app_data(installation_ids.clone())
                    .app_data(queue.clone())
                    .route("/webhook/github", web::post().to(github_webhook))
                    .route("/deploy", web::post().to(deploy_handler))
            })
            .bind(("127.0.0.1", 8080))?
            .run()
            .await?;
        }

        Commands::Serve {} => {
            let job_dispatcher = JobDispatcher::new(vm_pool.clone(), s3_service.clone());

            let vm_request_proxy = web::Data::new(Mutex::new(VmRequestProxy::new(
                vm_pool.clone(),
                id_allocator.clone(),
                job_dispatcher.clone(),
            )?));

            for _ in 0..1 {
                let id_allocator = id_allocator.clone();
                let vm_pool = vm_pool.clone();

                task::spawn(async move {
                    let new_id = id_allocator.allocate_id().await? as u32;
                    let mut new_vm = Firecracker::new(new_id);

                    new_vm.create_vm().await?;
                    vm_pool.add_to_ideal_vms(new_id);

                    Ok::<(), AppError>(())
                });
            }

            println!("Starting server");

            HttpServer::new(move || {
                App::new()
                    .app_data(vm_request_proxy.clone())
                    .route("/webhook/github", web::post().to(github_webhook))
                    .default_service(web::to(proxy))
            })
            .bind(("127.0.0.1", 8080))?
            .run()
            .await?;
        }

        Commands::Deploy {
            url,
            install,
            build,
            branch,
            dist_dir,
            home_dir,
        } => {
            let github_app_url = "https://github.com/apps/shipr-deployment/installations/new";

            println!(
                "Connect the project's repository with shipr by visiting. Opening the url in your browser..."
            );

            run_script(vec![&format!("xdg-open {}", github_app_url)], get_dir())?;

            println!("Waiting for installation...");

            let client = Client::new();

            let deploy_details = DeployReq {
                url,
                install,
                build,
                branch,
                home_dir,
                dist_dir,
            };

            let res = client
                .post("https://francisco-unscholarlike-punctually.ngrok-free.dev/deploy")
                .json(&deploy_details)
                .send()
                .await?;

            println!("Deploy response: {}", res.status());
        }

        Commands::Test => {
            let github = Github::new(3566236, "scrom-2511", "shipr_test_project");

            let token = github.get_installation_access_token().await?;

            println!("Token: {}", token);
        }
    }

    Ok(())
}
