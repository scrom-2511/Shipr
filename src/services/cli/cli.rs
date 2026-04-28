use actix_web::{App, HttpServer, web};
use clap::{Parser, Subcommand};
use tokio::task;

use crate::{
    app_errors::AppError,
    app_types::DeployDetails,
    services::{
        core::{pull_build::PullBuildCore, serve::ServeCore},
        firecracker::{
            firecracker::Firecracker, unique_id_allocator::UniqueIdAllocator, vm_pool::VmPool,
        },
        s3::s3::S3Service,
    },
    utils::{
        detect_project_type::ProjectType,
        run_script::{self, run_script},
    },
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
}

pub async fn cli(
    vm_pool: VmPool,
    id_allocator: UniqueIdAllocator,
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
            println!("URL: {}", url);
            println!("Install: {:?}", install);
            println!("Build: {:?}", build);

            let unique_id = uuid::Uuid::new_v4();
            println!("Unique ID: {}", unique_id);

            let presigned_url = s3_service
                .get_presigned_url(format!("{}.zip", unique_id).as_str())
                .await?;

            println!("Presigned URL: {}", presigned_url);

            let deploy_details = DeployDetails {
                url,
                install_commands: install,
                build_commands: build,
                branch,
                unique_id,
                home_dir,
                dist_dir,
                presigned_url,
            };

            println!("Worker copied to VM");

            let mut pull_build_core = PullBuildCore::new();
            pull_build_core
                .pull_build_setup(&deploy_details, id_allocator)
                .await?;
        }

        Commands::Serve {} => {
            let serve_core = web::Data::new(ServeCore::new(vm_pool.clone(), id_allocator.clone())?);

            for _ in 0..1 {
                let id_allocator = id_allocator.clone();
                let vm_pool = vm_pool.clone();

                task::spawn(async move {
                    let new_id = id_allocator.allocate_id().await? as u32;
                    let mut new_vm = Firecracker::new(new_id, ProjectType::Node);

                    new_vm.create_vm().await?;
                    vm_pool.add_to_ideal_vms(new_id);

                    Ok::<(), AppError>(())
                });
            }

            println!("Starting server");

            HttpServer::new(move || {
                App::new().app_data(serve_core.clone())
                // .default_service(web::to(proxy))
            })
            .bind(("127.0.0.1", 8080))?
            .run()
            .await?;
        }
    }

    Ok(())
}
