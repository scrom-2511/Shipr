pub mod is_logged_in;

pub struct AuthMiddleware {
    pub user_id: i32,
    pub email: String,
}
