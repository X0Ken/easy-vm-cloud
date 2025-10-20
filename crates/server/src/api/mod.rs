pub mod auth;
pub mod nodes;
pub mod vms;
pub mod user;
pub mod role;
pub mod permission;
pub mod department;
pub mod user_department;
pub mod utils;
pub mod storage;
pub mod networks;

use axum::{
    middleware::from_fn,
    Router,
};

use crate::{
    app_state::AppState,
    middleware::auth_middleware,
};

/// 所有 API 路由（统一入口）
pub fn api_routes() -> Router<AppState> {
    Router::new()
        // 不需要认证的路由
        .nest("/auth", auth::auth_routes())
        // 需要认证的路由
        .nest("/users", 
            user::user_routes()
                .layer(from_fn(auth_middleware))
        )
        .nest("/roles",
            role::role_routes()
                .layer(from_fn(auth_middleware))
        )
        .nest("/permissions",
            permission::permission_routes()
                .layer(from_fn(auth_middleware))
        )
        .nest("/departments",
            department::department_routes()
                .layer(from_fn(auth_middleware))
        )
        .nest("/user-departments",
            user_department::user_department_routes()
                .layer(from_fn(auth_middleware))
        )
        .nest("/nodes",
            nodes::node_routes()
                .layer(from_fn(auth_middleware))
        )
        .nest("/vms",
            vms::vm_routes()
                .layer(from_fn(auth_middleware))
        )
        .nest("/storage",
            storage::routes()
                .layer(from_fn(auth_middleware))
        )
        .nest("/networks",
            networks::routes()
                .layer(from_fn(auth_middleware))
        )
}