use actix_web::{HttpResponse, web};

use crate::{
    app_errors::AppError,
    app_types::{KillVmReq, LogsStore},
    controller::vm::{id_allocator::IdAllocator, vm_pool::VmPool},
    infra::kill_vm::kill_vm,
};

pub async fn kill_vm_handler(
    body: web::Bytes,
    vm_pool: web::Data<VmPool>,
    id_allocator: web::Data<IdAllocator>,
    logs_store: LogsStore,
) -> Result<HttpResponse, AppError> {
    let kill_vm_req = serde_json::from_slice::<KillVmReq>(&body).unwrap();

    println!("Kill VM request: {:?}", kill_vm_req);

    kill_vm(
        &kill_vm_req.project_id,
        &kill_vm_req.job_type,
        &vm_pool,
        &id_allocator,
    )
    .await?;

    logs_store.lock().await.remove(&kill_vm_req.project_id);

    Ok(HttpResponse::Ok().finish())
}
