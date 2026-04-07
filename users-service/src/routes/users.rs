use axum::extract::Path;

pub async fn get_all_users() -> &'static str {
    "All users endpoint"
}

pub async fn get_single_user(Path(id): Path<u32>) -> String {
    format!("User with id {}", id)
}
