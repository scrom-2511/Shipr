use actix_web::{HttpRequest, HttpResponse, Result, web};
use shipr::{
    app_errors::AppError,
    controller::{
        api::vm_request_proxy::VmRequestProxy,
        cli::cli::cli,
        storage::s3::S3Service,
        vm::{id_allocator::IdAllocator, vm_pool::VmPool},
    },
};

pub mod worker;

pub async fn proxy(
    core: web::Data<VmRequestProxy>,
    req: HttpRequest,
    body: web::Bytes,
) -> Result<HttpResponse, AppError> {
    core.proxy_request(req, body).await
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let vm_pool = VmPool::new();
    let id_allocator = IdAllocator::new();
    let s3_service = S3Service::new().await;

    cli(vm_pool, id_allocator, s3_service).await?;

    Ok(())
}
