use crate::{
    library::RecordingId,
    recorder::{Action, Recorder, Snapshot},
    rooms::{self, RoomId},
};
use axum::{
    Json, Router,
    extract::{Path, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::Deserialize;
use std::path::PathBuf;
use tower_http::{services::ServeDir, set_header::SetResponseHeaderLayer};

#[derive(Clone)]
pub struct AppState {
    pub recorder: Recorder,
    pub allow_test_sources: bool,
}

pub fn router(state: AppState, media: PathBuf, frontend: PathBuf) -> Router {
    Router::new()
        .route("/api/state", get(inspect))
        .route("/api/rooms", post(save_room))
        .route("/api/recordings", post(start))
        .route("/api/recordings/{id}/stop", post(stop))
        .route("/api/recordings/{id}/extend", post(extend))
        .nest_service("/media", ServeDir::new(media))
        .fallback_service(ServeDir::new(frontend))
        .layer(SetResponseHeaderLayer::overriding(
            header::CACHE_CONTROL,
            axum::http::HeaderValue::from_static("no-store"),
        ))
        .with_state(state)
}

struct HttpError(anyhow::Error);
impl From<anyhow::Error> for HttpError {
    fn from(error: anyhow::Error) -> Self {
        Self(error)
    }
}
impl IntoResponse for HttpError {
    fn into_response(self) -> Response {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": self.0.to_string() })),
        )
            .into_response()
    }
}
type HttpResult = Result<Json<Snapshot>, HttpError>;

async fn inspect(State(state): State<AppState>) -> HttpResult {
    Ok(Json(state.recorder.execute(Action::Inspect).await?))
}

#[derive(Deserialize)]
struct NewRoom {
    name: String,
    url: String,
}
async fn save_room(State(state): State<AppState>, Json(input): Json<NewRoom>) -> HttpResult {
    let room = rooms::resolve(input.name, input.url, state.allow_test_sources).await?;
    Ok(Json(state.recorder.execute(Action::SaveRoom(room)).await?))
}

#[derive(Deserialize)]
struct Start {
    room_id: RoomId,
    ends_at: i64,
}
async fn start(State(state): State<AppState>, Json(input): Json<Start>) -> HttpResult {
    Ok(Json(
        state
            .recorder
            .execute(Action::Start {
                room_id: input.room_id,
                ends_at: input.ends_at,
            })
            .await?,
    ))
}

// JSON bodies on mutations require application/json and prevent a plain
// cross-origin HTML form from issuing recorder commands. CORS stays disabled.
async fn stop(
    State(state): State<AppState>,
    Path(id): Path<RecordingId>,
    Json(_): Json<serde_json::Value>,
) -> HttpResult {
    Ok(Json(state.recorder.execute(Action::Stop(id)).await?))
}

#[derive(Deserialize)]
struct Extend {
    ends_at: i64,
}
async fn extend(
    State(state): State<AppState>,
    Path(id): Path<RecordingId>,
    Json(input): Json<Extend>,
) -> HttpResult {
    Ok(Json(
        state
            .recorder
            .execute(Action::Extend {
                id,
                ends_at: input.ends_at,
            })
            .await?,
    ))
}
