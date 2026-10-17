use axum::{
    Json, Router,
    extract::Path,
    http::StatusCode,
    routing::{delete, get, post, put},
};
use complex::core::application::use_case::{
    todo::{CreateTodoUseCase, DeleteTodoUseCase, GetAllTodoUseCase, UpdateStatusTodoUseCase},
    user::{CreateUserUseCase, DeleteUserUseCase, GetAllUserUseCase, GetByIdUserUseCase},
};
use complex::core::domain::todo::Todo;
use complex::core::domain::user::User;
use complex::infra::{di::RootModule, persistence::sqlite::SqliteClient};
use fluxdi::axum::{InjectorState, Resolved, request_scope};
use fluxdi::{
    Application, Error, Injector, Module, Provider, Shared, module::ModuleLifecycleFuture,
};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Debug, Serialize, Deserialize)]
struct CreateUserRequest {
    name: String,
    email: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct CreateTodoRequest {
    user_id: i64,
    title: String,
    description: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct UpdateTodoStatusRequest {
    completed: bool,
}

#[derive(Debug)]
struct RequestContext {
    request_scope_id: usize,
}

static NEXT_REQUEST_SCOPE_ID: AtomicUsize = AtomicUsize::new(1);

#[derive(Debug, Serialize)]
struct ApiResponse<T> {
    success: bool,
    data: Option<T>,
    error: Option<String>,
}

#[derive(Debug, Serialize)]
struct ScopedDebugResponse {
    request_scope_id: usize,
    same_instance_within_scope: bool,
}

impl<T: Serialize> ApiResponse<T> {
    fn ok(data: T) -> Self {
        ApiResponse {
            success: true,
            data: Some(data),
            error: None,
        }
    }
}

impl ApiResponse<()> {
    #[allow(unused)]
    fn error(msg: String) -> Self {
        ApiResponse {
            success: false,
            data: None,
            error: Some(msg),
        }
    }
}

// User Handlers
async fn create_user(
    Resolved(create_user): Resolved<CreateUserUseCase>,
    Json(req): Json<CreateUserRequest>,
) -> Result<(StatusCode, Json<ApiResponse<User>>), (StatusCode, String)> {
    let user = create_user
        .execute(req.name, req.email)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e))?;

    Ok((StatusCode::CREATED, Json(ApiResponse::ok(user))))
}

async fn get_all_users(
    Resolved(get_all): Resolved<GetAllUserUseCase>,
) -> Result<Json<ApiResponse<Vec<User>>>, (StatusCode, String)> {
    let users = get_all
        .execute()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    Ok(Json(ApiResponse::ok(users)))
}

async fn get_user_by_id(
    Resolved(get_by_id): Resolved<GetByIdUserUseCase>,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<User>>, (StatusCode, String)> {
    let user = get_by_id
        .execute(id as u32)
        .await
        .map_err(|e| (StatusCode::NOT_FOUND, e))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "User not found".to_string()))?;

    Ok(Json(ApiResponse::ok(user)))
}

async fn delete_user(
    Resolved(delete): Resolved<DeleteUserUseCase>,
    Path(id): Path<i64>,
) -> Result<(StatusCode, Json<ApiResponse<bool>>), (StatusCode, String)> {
    let deleted = delete
        .execute(id as u32)
        .await
        .map_err(|e| (StatusCode::NOT_FOUND, e))?;

    Ok((StatusCode::OK, Json(ApiResponse::ok(deleted))))
}

// Todo Handlers
async fn create_todo(
    Resolved(create_todo): Resolved<CreateTodoUseCase>,
    Json(req): Json<CreateTodoRequest>,
) -> Result<(StatusCode, Json<ApiResponse<Todo>>), (StatusCode, String)> {
    let todo = create_todo
        .execute(req.user_id as u32, req.title, req.description)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e))?;

    Ok((StatusCode::CREATED, Json(ApiResponse::ok(todo))))
}

async fn get_all_todos(
    Resolved(get_all): Resolved<GetAllTodoUseCase>,
) -> Result<Json<ApiResponse<Vec<Todo>>>, (StatusCode, String)> {
    let todos = get_all
        .execute()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    Ok(Json(ApiResponse::ok(todos)))
}

async fn update_todo_status(
    Resolved(update): Resolved<UpdateStatusTodoUseCase>,
    Path(id): Path<i64>,
    Json(req): Json<UpdateTodoStatusRequest>,
) -> Result<Json<ApiResponse<Todo>>, (StatusCode, String)> {
    let todo = update
        .execute(id as u32, req.completed)
        .await
        .map_err(|e| (StatusCode::NOT_FOUND, e))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Todo not found".to_string()))?;

    Ok(Json(ApiResponse::ok(todo)))
}

async fn delete_todo(
    Resolved(delete): Resolved<DeleteTodoUseCase>,
    Path(id): Path<i64>,
) -> Result<(StatusCode, Json<ApiResponse<bool>>), (StatusCode, String)> {
    let deleted = delete
        .execute(id as u32)
        .await
        .map_err(|e| (StatusCode::NOT_FOUND, e))?;

    Ok((StatusCode::OK, Json(ApiResponse::ok(deleted))))
}

async fn health_check() -> &'static str {
    "OK"
}

async fn scoped_debug(
    Resolved(context_a): Resolved<RequestContext>,
    Resolved(context_b): Resolved<RequestContext>,
) -> Json<ApiResponse<ScopedDebugResponse>> {
    Json(ApiResponse::ok(ScopedDebugResponse {
        request_scope_id: context_a.request_scope_id,
        same_instance_within_scope: Shared::ptr_eq(&context_a, &context_b),
    }))
}

fn build_router(state: InjectorState) -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/scoped/debug", get(scoped_debug))
        .route("/users", post(create_user))
        .route("/users", get(get_all_users))
        .route("/users/{id}", get(get_user_by_id))
        .route("/users/{id}", delete(delete_user))
        .route("/todos", post(create_todo))
        .route("/todos", get(get_all_todos))
        .route("/todos/{id}/status", put(update_todo_status))
        .route("/todos/{id}", delete(delete_todo))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            request_scope,
        ))
        .with_state(state)
}

struct WebApiModule;

impl Module for WebApiModule {
    fn imports(&self) -> Vec<Box<dyn Module>> {
        vec![Box::new(RootModule)]
    }

    fn configure(&self, injector: &Injector) -> Result<(), Error> {
        injector.provide::<SqliteClient>(Provider::root(|_| {
            let client = SqliteClient::new().expect("Failed to load sqlite client");
            Shared::new(client)
        }));
        injector.provide::<RequestContext>(Provider::scoped(|_| {
            Shared::new(RequestContext {
                request_scope_id: NEXT_REQUEST_SCOPE_ID.fetch_add(1, Ordering::SeqCst),
            })
        }));
        Ok(())
    }

    fn on_start(&self, injector: Shared<Injector>) -> ModuleLifecycleFuture {
        Box::pin(async move {
            let state = InjectorState::new(injector.clone());
            let app = build_router(state);

            let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
                .await
                .map_err(|err| {
                    Error::module_lifecycle_failed("WebApiModule", "on_start", &err.to_string())
                })?;

            println!("🚀 Server running on http://127.0.0.1:3000");
            println!("📚 Available endpoints:");
            println!("  GET    /health");
            println!("  GET    /scoped/debug");
            println!("  POST   /users");
            println!("  GET    /users");
            println!("  GET    /users/:id");
            println!("  DELETE /users/:id");
            println!("  POST   /todos");
            println!("  GET    /todos");
            println!("  PUT    /todos/:id/status");
            println!("  DELETE /todos/:id");

            axum::serve(listener, app).await.map_err(|err| {
                Error::module_lifecycle_failed("WebApiModule", "on_start", &err.to_string())
            })
        })
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    fluxdi::init_logging();

    let mut app = Application::new(WebApiModule);
    app.bootstrap().await
}
