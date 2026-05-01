use crate::app_errors::AppError;
use crate::controller::dispatcher::job_dispatcher::JobDispatcher;
use crate::controller::vm::id_allocator::IdAllocator;
use crate::controller::vm::vm_pool::VmPool;
use actix_web::http::Uri;
use actix_web::{HttpRequest, HttpResponse, web};
use reqwest::header::{HeaderName, HeaderValue};
use reqwest::{Client, Method};
use std::str::FromStr;
use std::time::Duration;
use tokio::net::TcpStream;
use url::Url;
use uuid::Uuid;

#[derive(Clone)]
pub struct VmRequestProxy {
    vm_pool: VmPool,
    id_allocator: IdAllocator,
    client: Client,
    job_dispatcher: JobDispatcher,
}

impl VmRequestProxy {
    pub fn new(
        vm_pool: VmPool,
        id_allocator: IdAllocator,
        job_dispatcher: JobDispatcher,
    ) -> Result<Self, AppError> {
        let client = Client::new();

        Ok(Self {
            vm_pool,
            id_allocator,
            client,
            job_dispatcher,
        })
    }

    fn extract_project_and_path(&self, req: &HttpRequest) -> Result<(Uuid, Uri), AppError> {
        let host = req.connection_info().host().to_string();

        let uri = req.uri().to_owned();

        let project_id = host
            .split(".")
            .next()
            .unwrap()
            .parse::<Uuid>()
            .map_err(|e| AppError::InvalidProjectId(e.to_string()))?;

        Ok((project_id, uri))
    }

    fn build_target_url(&self, vm_id: u32, uri: Uri) -> Url {
        let path = uri.path().trim_start_matches('/');
        let target_url_str = format!("http://172.16.0.{}:3000/{}", vm_id + 2, path);
        Url::from_str(&target_url_str).unwrap()
    }

    async fn forward_request(
        &self,
        req: &HttpRequest,
        body: web::Bytes,
        target_url: Url,
    ) -> HttpResponse {
        let method = req
            .method()
            .as_str()
            .parse::<Method>()
            .unwrap_or(Method::GET);

        let mut forward_req = self.client.request(method, target_url);

        for (name, value) in req.headers().iter() {
            if let Ok(header_name) = HeaderName::from_bytes(name.as_str().as_bytes()) {
                if let Ok(header_value) = HeaderValue::from_bytes(value.as_bytes()) {
                    forward_req = forward_req.header(header_name, header_value);
                }
            }
        }

        let resp = forward_req.body(body).send().await;

        match resp {
            Ok(upstream) => {
                let status = actix_web::http::StatusCode::from_u16(upstream.status().as_u16())
                    .unwrap_or(actix_web::http::StatusCode::INTERNAL_SERVER_ERROR);

                let mut response = HttpResponse::build(status);

                for (name, value) in upstream.headers().iter() {
                    if let Ok(value_str) = value.to_str() {
                        response.insert_header((name.as_str(), value_str));
                    }
                }

                match upstream.bytes().await {
                    Ok(bytes) => response.body(bytes),
                    Err(_) => {
                        HttpResponse::InternalServerError().body("Failed to read upstream body")
                    }
                }
            }
            Err(_) => HttpResponse::BadGateway().body("Upstream request failed"),
        }
    }

    pub async fn proxy_request(
        &mut self,
        req: HttpRequest,
        body: web::Bytes,
    ) -> Result<HttpResponse, AppError> {
        let (project_id, target_path) = self.extract_project_and_path(&req)?;

        self.job_dispatcher.dispatch_run_job(project_id).await?;

        println!("Job dispatched");

        let vm_id = self
            .vm_pool
            .get_from_pool(project_id)
            .ok_or(AppError::NoAvailableVm)?;

        self.wait_for_port(vm_id + 2, 3000, Duration::from_secs(30))
            .await?;

        // let id_allocator = id_allocator.clone();
        // let vm_pool = vm_pool.clone();

        // task::spawn(async move {
        //     let new_id = id_allocator.allocate_id().await? as u32;
        //     let mut new_vm = Firecracker::new(new_id, ProjectType::Node);

        //     new_vm.create_vm().await?;
        //     vm_pool.add_to_ideal_vms(new_id);

        //     Ok::<(), AppError>(())
        // });

        let target_url = self.build_target_url(vm_id, target_path);

        let resp = self.forward_request(&req, body, target_url).await;

        Ok(resp)
    }

    async fn wait_for_port(
        &self,
        vm_ip: u32,
        port: u16,
        max_wait: Duration,
    ) -> Result<(), AppError> {
        let addr = format!("172.16.0.{}:{}", vm_ip, port);
        let deadline = tokio::time::Instant::now() + max_wait;

        loop {
            if tokio::time::Instant::now() > deadline {
                return Err(AppError::StartingFirecrackerFailed(format!(
                    "Timed out waiting for port {}",
                    addr
                )));
            }
            match TcpStream::connect(&addr).await {
                Ok(_) => {
                    println!("Port ready: {}", addr);
                    return Ok(());
                }
                Err(_) => tokio::time::sleep(Duration::from_millis(200)).await,
            }
        }
    }
}
