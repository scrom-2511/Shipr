use std::{collections::HashMap, sync::Arc, thread::sleep, time::Duration};

use actix_web::{App, HttpRequest, HttpResponse, HttpServer, web};
use clap::{Parser, Subcommand};
use futures::lock::Mutex;
use tokio::task;

use crate::{
    app_errors::AppError,
    app_types::{DeployDetails, InstallationEvent},
    config::app_config::get_dir,
    controller::{
        api::{github::Github, vm_request_proxy::VmRequestProxy},
        dispatcher::job_dispatcher::JobDispatcher,
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

async fn github_webhook(
    body: web::Bytes,
    installation_ids: web::Data<Mutex<HashMap<String, InstallationEvent>>>,
) -> Result<HttpResponse, AppError> {
    let installation_event = serde_json::from_slice::<InstallationEvent>(&body).unwrap();
    if installation_event.action == "created" {
        println!(
            "Installation created: {:?}",
            installation_event.installation.id
        );
    };

    let url = format!(
        "https://github.com/{}",
        installation_event.repositories.full_name
    );

    installation_ids
        .lock()
        .await
        .insert(url, installation_event);

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
    // Deploy,
    Serve,
    Listen,
}

pub async fn cli(
    vm_pool: VmPool,
    id_allocator: IdAllocator,
    s3_service: S3Service,
) -> Result<(), AppError> {
    let args = Cli::parse();

    let installation_ids = Mutex::new(HashMap::<String, Arc<InstallationEvent>>::new());

    match args.command {
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

            sleep(Duration::from_secs(2));

            run_script(vec![&format!("xdg-open {}", github_app_url)], get_dir())?;

            println!("Waiting for installation...");

            let url = url.replace(".git", "");

            let installation_event = loop {
                if let Some(installation_event) = installation_ids.lock().await.get(&url) {
                    break installation_event.clone();
                }
                sleep(Duration::from_secs(1));
            };

            let github = Github::new("3566236".to_string());

            let token = github
                .get_installation_access_token(&installation_event.installation.id)
                .await?;

            let url = format!(
                "https://x-access-token:{}@github.com/{}.git",
                token, installation_event.repositories.full_name
            );

            println!("Access Token: {}", token);

            let project_id = uuid::Uuid::new_v4();

            let presigned_upload_url = s3_service
                .get_presigned_upload_url(&project_id.to_string())
                .await?;

            for _ in 0..1 {
                let new_id = id_allocator.allocate_id().await? as u32;
                let mut new_vm = Firecracker::new(new_id);

                new_vm.create_vm().await?;
                vm_pool.add_to_ideal_vms(new_id);
            }

            let deploy_details = DeployDetails {
                url,
                install_commands: install,
                build_commands: build,
                branch,
                project_id,
                home_dir,
                dist_dir,
                presigned_upload_url,
            };

            let mut job_dispatcher = JobDispatcher::new(vm_pool, s3_service);
            job_dispatcher.dispatch_deploy_job(&deploy_details).await?;
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

        Commands::Listen {} => {
            let installation_ids_data = web::Data::new(installation_ids);

            HttpServer::new(move || {
                App::new()
                    .app_data(installation_ids_data.clone())
                    .route("/webhook/github", web::post().to(github_webhook))
            })
            .bind(("127.0.0.1", 8080))?
            .run()
            .await?;
        }
    }

    Ok(())
}
