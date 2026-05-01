use shipr::controller::{
    cli::cli::cli,
    storage::s3::S3Service,
    vm::{id_allocator::IdAllocator, vm_pool::VmPool},
};

pub mod worker;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let vm_pool = VmPool::new();
    let id_allocator = IdAllocator::new();
    let s3_service = S3Service::new().await;

    cli(vm_pool, id_allocator, s3_service).await?;

    Ok(())
}
