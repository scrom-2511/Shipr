use actix_web::{App, HttpRequest, HttpResponse, HttpServer, web};
use futures::lock::Mutex;
use tokio::task;

use crate::{
    app_errors::AppError,
    core::controller::{
        api::vm_request_proxy::VmRequestProxy,
        dispatcher::job_dispatcher::JobDispatcher,
        storage::s3::S3Service,
        vm::{
            firecracker::Firecracker, heartbeat_store::HeartbeatStore, id_allocator::IdAllocator,
            vm_pool::VmPool,
        },
    },
};

pub async fn proxy(
    vm_request_proxy: web::Data<Mutex<VmRequestProxy>>,
    req: HttpRequest,
    body: web::Bytes,
) -> Result<HttpResponse, AppError> {
    vm_request_proxy.lock().await.proxy_request(req, body).await
}

pub async fn serve(
    id_allocator: IdAllocator,
    vm_pool: VmPool,
    s3_service: S3Service,
    heartbeat_store: HeartbeatStore,
) -> Result<(), AppError> {
    let job_dispatcher = JobDispatcher::new(
        vm_pool.clone(),
        s3_service.clone(),
        id_allocator.clone(),
        heartbeat_store.clone(),
    );

    let vm_request_proxy = web::Data::new(Mutex::new(VmRequestProxy::new(
        vm_pool.clone(),
        job_dispatcher.clone(),
        heartbeat_store.clone(),
    )?));

    for _ in 0..1 {
        let id_allocator = id_allocator.clone();

        let vm_pool = vm_pool.clone();

        task::spawn(async move {
            let mut new_vm = Firecracker::new_from_id_allocator(&id_allocator).await;
            new_vm.create_new_vm_and_add_to_pool(&vm_pool).await?;

            Ok::<(), AppError>(())
        });
    }

    println!("Starting server");

    HttpServer::new(move || {
        App::new()
            .app_data(vm_request_proxy.clone())
            .default_service(web::to(proxy))
    })
    .bind(("127.0.0.1", 3001))?
    .run()
    .await?;

    Ok(())
}
