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
    app_types::{
        DeployDetails, DeployReq, EventType, InstallationEvent, RedeployDetails, RedeployEvent,
    },
    config::app_config::get_dir,
    controller::{
        api::{github::Github, vm_request_proxy::VmRequestProxy},
        cli::{deploy::deploy, listen::listen, serve::serve},
        dispatcher::job_dispatcher::JobDispatcher,
        queue::{deploy_queue::DeployQueue, lapin::Lapin, redeploy_queue::ReDeployQueue},
        storage::s3::S3Service,
        vm::{firecracker::Firecracker, id_allocator::IdAllocator, vm_pool::VmPool},
    },
    infra::process::run_script,
};

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

    match args.command {
        Commands::Listen => {
            println!("Starting listener...");
            listen(id_allocator, vm_pool, s3_service).await?;
        }

        Commands::Serve => {
            serve(vm_pool, id_allocator, s3_service).await?;
        }

        Commands::Deploy {
            url,
            install,
            build,
            branch,
            dist_dir,
            home_dir,
        } => {
            let deploy_req = DeployReq {
                branch,
                build,
                dist_dir,
                home_dir,
                install,
                url,
            };

            deploy(deploy_req).await?;
        }

        Commands::Test => {
            let github = Github::new(3566236, "scrom-2511", "shipr_test_project");

            let token = github.get_installation_access_token().await?;

            println!("Token: {}", token);
        }
    }

    Ok(())
}
