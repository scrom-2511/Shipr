use actix_web::{HttpResponse, web};
use serde_json::Value;

use crate::{
    app_errors::AppError, core::app_types::JobType, core::controller::vm::vm_pool::VmPool,
};

pub async fn redeploy_completed_handler(
    body: web::Bytes,
    vm_pool: web::Data<VmPool>,
) -> Result<HttpResponse, AppError> {
    let redeploy_details = serde_json::from_slice::<Value>(&body).unwrap();

    let project_id = redeploy_details["project_id"].as_str().unwrap();

    vm_pool
        .remove_from_pool(project_id, &JobType::Redeploy)
        .await?;

    Ok(HttpResponse::Ok().finish())
}
