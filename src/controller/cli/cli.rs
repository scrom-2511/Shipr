use std::{thread::sleep, time::Duration};

use actix_web::{App, HttpRequest, HttpResponse, HttpServer, web};
use clap::{Parser, Subcommand};
use futures::lock::Mutex;
use tokio::task;

use crate::{
    app_errors::AppError,
    app_types::{DeployDetails, ProjectType},
    controller::{
        api::vm_request_proxy::VmRequestProxy,
        dispatcher::job_dispatcher::JobDispatcher,
        storage::s3::S3Service,
        vm::{firecracker::Firecracker, id_allocator::IdAllocator, vm_pool::VmPool},
    },
};

pub async fn proxy(
    vm_request_proxy: web::Data<Mutex<VmRequestProxy>>,
    req: HttpRequest,
    body: web::Bytes,
) -> Result<HttpResponse, AppError> {
    vm_request_proxy.lock().await.proxy_request(req, body).await
}

async fn github_webhook(body: web::Bytes) -> HttpResponse {
    println!("Webhook received: {:?}", body);
    HttpResponse::Ok().finish()
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
}

pub async fn cli(
    vm_pool: VmPool,
    id_allocator: IdAllocator,
    s3_service: S3Service,
) -> Result<(), AppError> {
    let args = Cli::parse();

    match args.command {
        Commands::Deploy {
            url,
            install,
            build,
            branch,
            dist_dir,
            home_dir,
        } => {
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
    }

    Ok(())
}
