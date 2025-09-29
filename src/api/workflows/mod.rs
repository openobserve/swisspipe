pub mod types;
pub mod validation;
pub mod operations;
pub mod utils;
pub mod handlers;
pub mod service;

use axum::Router;
use crate::AppState;

#[allow(unused_imports)]
pub use types::*;
pub use handlers::{list_workflows, create_workflow, get_workflow, delete_workflow, enable_workflow, update_workflow, search_workflows};

pub fn routes() -> Router<AppState> {
    #[allow(unused_imports)]
    use axum::routing::{delete, get, post, put};
    
    Router::new()
        .route("/", get(list_workflows).post(create_workflow))
        .route("/search", get(search_workflows))
        .route("/:id", get(get_workflow).put(update_workflow).delete(delete_workflow))
        .route("/:id/enable", put(enable_workflow))
}