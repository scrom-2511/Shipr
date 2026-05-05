use reqwest::Client;

use crate::{
    app_errors::AppError, app_types::DeployReq, config::app_config::get_dir,
    infra::process::run_script,
};

pub async fn deploy(deploy_req: DeployReq) -> Result<(), AppError> {
    let github_app_url = "https://github.com/apps/shipr-deployment/installations/new";

    println!(
        "Connect the project's repository with shipr by visiting. Opening the url in your browser..."
    );

    run_script(vec![&format!("xdg-open {}", github_app_url)], get_dir())?;

    println!("Waiting for installation...");

    let client = Client::new();

    let res = client
        .post("https://francisco-unscholarlike-punctually.ngrok-free.dev/deploy")
        .json(&deploy_req)
        .send()
        .await?;

    println!("Deploy response: {}", res.status());
    Ok(())
}
