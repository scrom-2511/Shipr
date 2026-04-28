use actix_web::{HttpRequest, HttpResponse, Result, web};
use shipr::{
    app_errors::AppError,
    services::{
        cli::cli::cli,
        core::serve::ServeCore,
        firecracker::{unique_id_allocator::UniqueIdAllocator, vm_pool::VmPool},
        s3::s3::S3Service,
    },
};

pub mod worker;

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
    let s3_service = S3Service::new().await;

    cli(vm_pool, id_allocator, s3_service).await?;

    Ok(())
}
