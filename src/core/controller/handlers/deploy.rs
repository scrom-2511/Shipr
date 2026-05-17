use std::fs;

use actix_web::{
    HttpResponse,
    web::{self},
};
use tokio::sync::broadcast::channel;

use crate::{
    app_errors::AppError,
    core::app_types::{DeployReq, LogsStore},
    core::controller::queue::deploy_queue::DeployQueue,
};

pub async fn deploy_handler(
    body: web::Json<DeployReq>,
    deploy_queue: web::Data<DeployQueue>,
    logs_store: LogsStore,
) -> Result<HttpResponse, AppError> {
    let deploy_details = body.into_inner();

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

    logs_store.lock().await.insert(project_id.clone(), tx);

    println!("{:?}", logs_store.lock().await.keys());

    deploy_queue.add_to_queue(&deploy_details).await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "project_id": project_id
    })))
}
