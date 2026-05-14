use std::error::Error;

use actix_web::{App, HttpServer, web};
use dotenv::dotenv;
use shipr::app::controllers::auth::github_signup::{github_auth_url, github_callback};
use shipr::app::controllers::auth::signin::signin_controller;
use shipr::app::controllers::auth::signup::signup_controller;
use shipr::app::controllers::project::add_new_project::add_new_project;
use shipr::app::controllers::project::deploy_project::deploy_project_controller;
use shipr::app::controllers::project::get_all_projects::get_all_projects;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv().ok();

    let db_url = "postgresql://neondb_owner:npg_RlYICzb47Sps@ep-weathered-mode-aqn5uvc3-pooler.c-8.us-east-1.aws.neon.tech/neondb?sslmode=require&channel_binding=require";

    let pool = sqlx::postgres::PgPool::connect(&db_url).await?;

    sqlx::migrate!("src/app/migrations").run(&pool).await?;

    println!("Server running on port 9000");

    HttpServer::new(move || {
        let cors = actix_cors::Cors::default()
            .allowed_origin("http://localhost:5173")
            .allow_any_method()
            .allow_any_header()
            .max_age(3600);

        App::new()
            .wrap(cors)
            .app_data(web::Data::new(pool.clone()))
            .route("/signup", web::post().to(signup_controller))
            .route("/signin", web::post().to(signin_controller))
            .route("/deploy-project", web::post().to(deploy_project_controller))
            .route("/get-projects", web::get().to(get_all_projects))
            .route("/add-project", web::post().to(add_new_project))
            .route("/auth/github", web::get().to(github_auth_url))
            .route("/auth/github/callback", web::get().to(github_callback))
    })
    .bind(("127.0.0.1", 9000))?
    .run()
    .await?;

    Ok(())
}
