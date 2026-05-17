use std::net::UdpSocket;

use clap::{Parser, Subcommand};

use crate::{
    app_errors::AppError,
    core::app_types::DeployReq,
    core::controller::{
        cli::{deploy::deploy, listen::listen, serve::serve},
        storage::s3::S3Service,
        vm::{heartbeat_store::HeartbeatStore, id_allocator::IdAllocator, vm_pool::VmPool},
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
        run: Vec<String>,

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
    heartbeat_store: HeartbeatStore,
) -> Result<(), AppError> {
    let args = Cli::parse();

    match args.command {
        Commands::Listen => {
            println!("Starting listener...");
            listen(id_allocator, vm_pool, s3_service, heartbeat_store).await?;
        }

        Commands::Serve => {
            serve(id_allocator, vm_pool, s3_service, heartbeat_store).await?;
        }

        Commands::Deploy {
            url,
            install,
            build,
            run,
            branch,
            dist_dir,
            home_dir,
        } => {
            let deploy_req = DeployReq {
                branch: Some(branch),
                build: Some(build),
                run: Some(run),
                dist_dir,
                home_dir,
                install: Some(install),
                url,
                full_name,
            };

            deploy(deploy_req).await?;
        }

        Commands::Test => {
            // let vec = vec![0, 1, 2, 3, 4, 5, 6];

            // for i in vec {
            //     let firecracker = Firecracker::new(i);
            //     firecracker.destroy_vm().await?;
            //     println!("VM {} destroyed", i);
            // }

            let socket = UdpSocket::bind("0.0.0.0:0")?;

            socket.connect("8.8.8.8:80")?;

            let local_ip = socket.local_addr()?.ip();

            println!("Default IP: {}", local_ip);
        }
    }

    Ok(())
}
