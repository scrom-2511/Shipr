use actix_web::{App, HttpRequest, HttpResponse, HttpServer, web};
use futures::lock::Mutex;
use tokio::task;

use crate::{
    app_errors::AppError,
    controller::{
        api::vm_request_proxy::VmRequestProxy,
        dispatcher::job_dispatcher::JobDispatcher,
        storage::s3::S3Service,
        vm::{firecracker::Firecracker, id_allocator::IdAllocator, vm_pool::VmPool},
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
    vm_pool: VmPool,
    id_allocator: IdAllocator,
    s3_service: S3Service,
) -> Result<(), AppError> {
    let job_dispatcher = JobDispatcher::new(vm_pool.clone(), s3_service.clone());

    let vm_request_proxy = web::Data::new(Mutex::new(VmRequestProxy::new(
        vm_pool.clone(),
        id_allocator.clone(),
        job_dispatcher.clone(),
    )?));

    for _ in 0..1 {
        let id_allocator = id_allocator.clone();
        let vm_pool = vm_pool.clone();

        task::spawn(async move {
            let new_id = id_allocator.allocate_id().await? as u32;
            let mut new_vm = Firecracker::new(new_id);

            new_vm.create_vm().await?;
            vm_pool.add_to_ideal_vms(new_id);

            Ok::<(), AppError>(())
        });
    }

    println!("Starting server");

    HttpServer::new(move || {
        App::new()
            .app_data(vm_request_proxy.clone())
            .default_service(web::to(proxy))
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await?;

    Ok(())
}
