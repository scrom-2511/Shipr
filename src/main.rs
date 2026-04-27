use actix_web::{App, HttpRequest, HttpResponse, HttpServer, Result, web};
use tokio::task;

pub mod app_errors;
pub mod app_types;
pub mod config;
pub mod services;
pub mod utils;

use crate::app_errors::AppError;
use crate::services::cli::cli::deploy;
use crate::services::core::serve::ServeCore;
use crate::services::firecracker::firecracker::Firecracker;
use crate::services::firecracker::unique_id_allocator::UniqueIdAllocator;
use crate::services::firecracker::vm_pool::VmPool;
use crate::utils::detect_project_type::ProjectType;

pub async fn proxy(
    core: web::Data<ServeCore>,
    req: HttpRequest,
    body: web::Bytes,
) -> Result<HttpResponse, AppError> {
    core.proxy_request(req, body).await
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let vm_pool = VmPool::new();
    let id_allocator = UniqueIdAllocator::new();

    deploy(vm_pool, id_allocator).await?;

    Ok(())
}
