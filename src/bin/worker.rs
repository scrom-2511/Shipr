use std::fs;

use shipr::{app_types::DeployDetails, worker::job_executer::PullBuildWorker};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: worker <job.json>");
        std::process::exit(1);
    }

    let json_path = &args[1];

    let content = fs::read_to_string(json_path).unwrap();

    let deploy_details = serde_json::from_str::<DeployDetails>(&content).unwrap();

    let worker = PullBuildWorker::new();

    worker.pull_build(&deploy_details).await?;

    println!("✅ Build and upload completed");

    Ok(())
}
