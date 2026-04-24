use std::thread::spawn;
use std::time::Duration;

use crate::services::firecracker::unique_id_allocator::UniqueIdAllocator;
use crate::services::firecracker::vm_pool::VmPool;
use crate::utils::run_script::run_script;
use crate::{
    app_errors::AppError,
    services::firecracker::firecracker::Firecracker,
    utils::detect_project_type::{ProjectType, detect_project_type},
};
use actix_web::web::Bytes;
use actix_web::{HttpRequest, HttpResponse, web};
use reqwest::Client;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use tokio::net::TcpStream;
use tokio::task;
use uuid::Uuid;

pub struct ServeCore {
    vm_pool: web::Data<VmPool>,
    id_allocator: web::Data<UniqueIdAllocator>,
    client: Client,
}

impl ServeCore {
    pub fn new(
        vm_pool: web::Data<VmPool>,
        id_allocator: web::Data<UniqueIdAllocator>,
    ) -> Result<Self, AppError> {
        let client = Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|e| AppError::HttpClientBuildFailed(e.to_string()))?;

        Ok(Self {
            vm_pool,
            id_allocator,
            client,
        })
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

    fn convert_headers(&self, actix_headers: &actix_web::http::header::HeaderMap) -> HeaderMap {
        let mut headers = HeaderMap::new();
        for (name, value) in actix_headers {
            let name = HeaderName::from_bytes(name.as_str().as_bytes()).unwrap();
            let value = HeaderValue::from_bytes(value.as_bytes()).unwrap();
            headers.insert(name, value);
        }
        headers
    }

    async fn proxy_request(
        &self,
        req: HttpRequest,
        body: web::Bytes,
    ) -> Result<HttpResponse, AppError> {
        let (project_id, target_path, query) = self.extract_project_and_path(&req)?;

        let vm_id = self.get_or_create_vm(project_id).await?;

        let target_url = self.build_target_url(vm_id, &target_path, &query);

        let resp = self.forward_request(&req, body, &target_url).await?;

        self.build_response(resp, vm_id, project_id).await
    }

    fn extract_project_and_path(
        &self,
        req: &HttpRequest,
    ) -> Result<(Uuid, String, String), AppError> {
        let path = req.uri().to_string();
        let path = path.strip_prefix("/").unwrap_or("");

        let mut parts = path.splitn(2, '/');

        let project_id = parts.next().unwrap_or("");
        let project_id = Uuid::parse_str(project_id).map_err(|_| AppError::InvalidProjectId)?;

        println!("Project ID: {}", project_id);

        let remaining = parts.next().unwrap_or("");
        let target_path = if remaining.is_empty() {
            "".to_string()
        } else {
            remaining.to_string()
        };

        let query = req
            .uri()
            .query()
            .map(|q| format!("?{}", q))
            .unwrap_or_default();

        Ok((project_id, target_path, query))
    }

    async fn get_or_create_vm(&self, project_id: Uuid) -> Result<u32, AppError> {
        match self.vm_pool.get_from_pool(project_id) {
            Some(id) => {
                println!("Using existing VM {}", id);
                Ok(id)
            }
            None => {
                let new_id = self
                    .vm_pool
                    .get_from_ideal_vms()
                    .ok_or(AppError::NoAvailableVm)?;

                println!("Starting VM {}", new_id);

                self.serve(
                    project_id,
                    new_id as u32,
                    vec![format!("cd {} && npx serve dist", project_id)],
                    self.vm_pool.clone(),
                    self.id_allocator.clone(),
                )
                .await
                .map_err(|e| AppError::VmProvisioningFailed(e.to_string()))?;

                Ok(new_id as u32)
            }
        }
    }

    fn build_target_url(&self, vm_id: u32, target_path: &str, query: &str) -> String {
        let url = format!(
            "http://172.16.0.{}:3000/{}{}",
            vm_id + 2,
            target_path,
            query
        );

        println!("Target URL: {}", url);
        url
    }

    async fn forward_request(
        &self,
        req: &HttpRequest,
        body: web::Bytes,
        target_url: &str,
    ) -> Result<reqwest::Response, AppError> {
        let method = req
            .method()
            .as_str()
            .parse::<reqwest::Method>()
            .map_err(|e| AppError::MethodConversionFailed(e.to_string()))?;

        let headers = self.convert_headers(req.headers());

        let resp = self
            .client
            .request(method, target_url)
            .headers(headers)
            .body(body.to_vec())
            .send()
            .await
            .map_err(|e| AppError::RequestForwardingFailed(e.to_string()))?;

        Ok(resp)
    }

    async fn build_response(
        &self,
        resp: reqwest::Response,
        vm_id: u32,
        project_id: Uuid,
    ) -> Result<HttpResponse, AppError> {
        let status = resp.status();
        let resp_headers = resp.headers().clone();

        let mut response_builder = HttpResponse::build(
            status
                .as_u16()
                .try_into()
                .unwrap_or(actix_web::http::StatusCode::OK),
        );

        // Header rewriting
        for (name, value) in &resp_headers {
            if name == reqwest::header::LOCATION {
                if let Ok(loc) = value.to_str() {
                    let rewritten = loc.replace(
                        &format!("http://172.16.0.{}:3000", vm_id + 2),
                        &format!("http://127.0.0.1:8080/{}", project_id),
                    );
                    response_builder.insert_header((name.as_str(), rewritten));
                    continue;
                }
            }
            response_builder.insert_header((name.as_str(), value.as_bytes()));
        }

        let bytes = resp
            .bytes()
            .await
            .map_err(|e| AppError::ResponseReadFailed(e.to_string()))?;

        let content_type = resp_headers
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();

        let final_bytes = if content_type.contains("text/html") {
            self.rewrite_html(bytes, project_id)
        } else {
            bytes.to_vec()
        };

        Ok(response_builder.body(final_bytes))
    }

    fn rewrite_html(&self, bytes: Bytes, project_id: Uuid) -> Vec<u8> {
        let html = String::from_utf8_lossy(&bytes);

        let strip_prefix_script = format!(
            r#"<script>(function(){{var p=window.location.pathname;var prefix='/{0}';if(p.startsWith(prefix)){{var newPath=p.slice(prefix.length)||'/';history.replaceState(null,'',newPath+window.location.search+window.location.hash);}}}})();</script>"#,
            project_id
        );

        let rewritten = html
            .replace("src=\"/", &format!("src=\"/{}/", project_id))
            .replace("href=\"/", &format!("href=\"/{}/", project_id))
            .replace("<head>", &format!("<head>{}", strip_prefix_script));

        rewritten.into_bytes()
    }

    pub async fn proxy_entry(
        handler: web::Data<ServeCore>,
        req: HttpRequest,
        body: web::Bytes,
    ) -> Result<HttpResponse, AppError> {
        handler.proxy_request(req, body).await
    }
    pub async fn serve(
        &self,
        project_id: Uuid,
        vm_id: u32,
        run_script_vm: Vec<String>,
        vm_pool: web::Data<VmPool>,
        id_allocator: web::Data<UniqueIdAllocator>,
    ) -> Result<(), AppError> {
        let project_type = detect_project_type(&project_id.to_string());

        match project_type {
            ProjectType::Unknown => {
                return Err(AppError::UnknownProjectType);
            }
            _ => {
                let new_vm = Firecracker::new(vm_id, ProjectType::Node);
                vm_pool.add_to_pool(project_id, vm_id);

                let copy_dist_dir_to_microvm = format!(
                    "scp -r -i ubuntu.id_rsa /home/scrom/code/shipr/build/{} root@172.16.0.{}:/root/{}",
                    project_id,
                    vm_id + 2,
                    project_id
                );

                run_script(vec![&copy_dist_dir_to_microvm])?;

                let run_script_final = run_script_vm.join(" && ");

                println!("Executing cmd {}", run_script_final);

                spawn(move || new_vm.execute_command(&run_script_final));

                println!("Executing cmd done");

                self.wait_for_port(vm_id + 2, 3000, Duration::from_secs(30))
                    .await?;
            }
        }

        let id_allocator = id_allocator.clone();
        let vm_pool = vm_pool.clone();

        task::spawn(async move {
            let new_id = id_allocator.allocate_id().await? as u32;
            let mut new_vm = Firecracker::new(new_id, ProjectType::Node);

            new_vm.create_vm().await?;
            vm_pool.add_to_ideal_vms(new_id);

            Ok::<(), AppError>(())
        });

        Ok(())
    }
}
