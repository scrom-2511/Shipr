use std::error::Error;

use actix_web::{App, HttpServer, web};
use dotenv::dotenv;
use shipr::app::controllers::auth::signup::signup_controller;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv().ok();

    let db_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:password@localhost/shipr".to_string());

    let pool = sqlx::postgres::PgPool::connect(&db_url).await?;

    sqlx::migrate!("src/app/migrations").run(&pool).await?;

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .route("/signup", web::post().to(signup_controller))
    })
    .bind(("127.0.0.1", 9000))?
    .run()
    .await?;

    Ok(())
}
