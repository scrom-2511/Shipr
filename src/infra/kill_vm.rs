use crate::app_errors::AppError;
use crate::app_types::JobType;
use crate::controller::vm::firecracker::Firecracker;
use crate::controller::vm::id_allocator::IdAllocator;
use crate::controller::vm::vm_pool::VmPool;

pub async fn kill_vm(
    project_id: &str,
    job_type: &JobType,
    vm_pool: &VmPool,
    id_allocator: &IdAllocator,
) -> Result<(), AppError> {
    let vm_id = vm_pool.get_from_pool(&project_id, job_type).await?.unwrap();

    let new_vm = Firecracker::new(vm_id);

    new_vm.destroy_vm().await?;
    vm_pool.remove_from_pool(&project_id, job_type).await?;
    id_allocator.release_id(vm_id).await?;

    Ok(())
}
