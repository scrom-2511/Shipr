use actix_web::{App, HttpRequest, HttpResponse, HttpServer, Result, web};
use reqwest::Client;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use tokio::task;
use uuid::Uuid;

pub mod app_errors;
pub mod app_types;
pub mod services;
pub mod utils;

use crate::app_errors::AppError;
use crate::services::core::serve::ServeCore;
use crate::services::firecracker::firecracker::Firecracker;
use crate::services::firecracker::unique_id_allocator::UniqueIdAllocator;
use crate::services::firecracker::vm_pool::VmPool;
use crate::utils::detect_project_type::ProjectType;

fn convert_headers(actix_headers: &actix_web::http::header::HeaderMap) -> HeaderMap {
    let mut headers = HeaderMap::new();
    for (name, value) in actix_headers {
        let name = HeaderName::from_bytes(name.as_str().as_bytes()).unwrap();
        let value = HeaderValue::from_bytes(value.as_bytes()).unwrap();
        headers.insert(name, value);
    }
    headers
}

async fn proxy_request(
    req: HttpRequest,
    body: web::Bytes,
    vm_pool: web::Data<VmPool>,
) -> Result<HttpResponse, AppError> {
    let client = Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| AppError::HttpClientBuildFailed(e.to_string()))?;

    let path = req.uri().to_string();
    let path = path.strip_prefix("/").unwrap_or("");

    let mut parts = path.splitn(2, '/');
    let project_id = parts.next().unwrap_or("");
    let project_id = Uuid::parse_str(project_id).map_err(|_| AppError::InvalidProjectId)?;
    println!("Project ID: {}", project_id);

    let vm_id = match vm_pool.get_from_pool(project_id) {
        Some(id) => {
            println!("Using existing VM {}", id);
            id
        }
        None => {
            let new_id = vm_pool.get_from_ideal_vms().unwrap();
            println!("Starting VM {}", new_id);

            ServeCore::new()
                .serve(
                    project_id,
                    new_id as u32,
                    vec![format!("cd {} && npx serve dist", project_id)],
                    vm_pool,
                )
                .await
                .map_err(|e| AppError::VmProvisioningFailed(e.to_string()))?;

            new_id as u32
        }
    };

    let remaining = parts.next().unwrap_or("");
    let target_path = if remaining.is_empty() { "" } else { remaining };

    let query = req
        .uri()
        .query()
        .map(|q| format!("?{}", q))
        .unwrap_or_default();

    let target_url = format!(
        "http://172.16.0.{}:3000/{}{}",
        vm_id + 2,
        target_path,
        query
    );

    println!("Target URL: {}", target_url);

    let method = req
        .method()
        .as_str()
        .parse::<reqwest::Method>()
        .map_err(|e| AppError::MethodConversionFailed(e.to_string()))?;

    let headers = convert_headers(req.headers());

    let resp = client
        .request(method, &target_url)
        .headers(headers)
        .body(body.to_vec())
        .send()
        .await
        .map_err(|e| AppError::RequestForwardingFailed(e.to_string()))?;

    let status = resp.status();
    let resp_headers = resp.headers().clone();

    let mut response_builder = HttpResponse::build(
        status
            .as_u16()
            .try_into()
            .unwrap_or(actix_web::http::StatusCode::OK),
    );

    // for (name, value) in &resp_headers {
    //     if name == reqwest::header::LOCATION {
    //         if let Ok(loc) = value.to_str() {
    //             let rewritten = loc.replace(
    //                 &format!("http://172.16.0.{}:3000", vm_id + 2),
    //                 &format!("http://127.0.0.1:8080/{}", project_id),
    //             );
    //             response_builder.insert_header((name.as_str(), rewritten));
    //             continue;
    //         }
    //     }
    //     response_builder.insert_header((name.as_str(), value.as_bytes()));
    // }

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
    } else {
        bytes.to_vec()
    };

    Ok(response_builder.body(final_bytes))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let id_allocator = UniqueIdAllocator::new();
    let vm_pool = VmPool::new();

    let mut handles = vec![];

    for vm_id in 0..1 {
        let id_allocator = id_allocator.clone();
        let vm_pool = vm_pool.clone();

        let handle = task::spawn(async move {
            let new_id = id_allocator.allocate_id().await? as u32;
            let mut new_vm = Firecracker::new(new_id, ProjectType::Node);

            new_vm.create_vm().await?;
            vm_pool.add_to_ideal_vms(new_id);

            Ok::<(), AppError>(())
        });

        handles.push(handle);
    }

    println!("Starting server");

    HttpServer::new(move || {
        App::new()
            .route("/{tail:.*}", web::to(proxy_request))
            .app_data(web::Data::new(vm_pool.clone()))
            .app_data(web::Data::new(id_allocator.clone()))
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await?;

    Ok(())
}
