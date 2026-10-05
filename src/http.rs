use crate::{
    downloads::Downloads,
    library::RecordingId,
    recorder::{Action, Recorder, Snapshot},
    rooms::{self, RoomId},
    subjects::SubjectId,
};
use axum::{
    Extension, Json, Router,
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
    pub ffmpeg: String,
    pub allow_test_sources: bool,
}

pub fn router(state: AppState, media: PathBuf, frontend: PathBuf) -> Router {
    let downloads = Downloads::new(media.clone(), state.ffmpeg.clone());
    Router::new()
        .route("/api/state", get(inspect))
        .route("/api/rooms", post(save_room))
        .route("/api/rooms/{id}/remove", post(remove_room))
        .route("/api/recordings", post(start))
        .route("/api/recordings/{id}/stop", post(stop))
        .route("/api/recordings/{id}/extend", post(extend))
        .route("/api/recordings/{id}/subject", post(assign_subject))
        .route("/api/recordings/{id}/download", get(download))
        .nest_service("/media", ServeDir::new(media))
        .fallback_service(ServeDir::new(frontend))
        .layer(SetResponseHeaderLayer::overriding(
            header::CACHE_CONTROL,
            axum::http::HeaderValue::from_static("no-store"),
        ))
        .with_state(state)
        .layer(Extension(downloads))
}

async fn download(
    State(state): State<AppState>,
    Extension(downloads): Extension<Downloads>,
    Path(id): Path<RecordingId>,
) -> Result<Response, HttpError> {
    let snapshot = state.recorder.execute(Action::Inspect).await?;
    let recording = snapshot
        .recordings
        .iter()
        .find(|recording| recording.id == id)
        .ok_or_else(|| HttpError(anyhow::anyhow!("Recording not found")))?;
    Ok(downloads.open(recording).await?)
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
    subject_id: Option<SubjectId>,
}
async fn start(State(state): State<AppState>, Json(input): Json<Start>) -> HttpResult {
    Ok(Json(
        state
            .recorder
            .execute(Action::Start {
                room_id: input.room_id,
                ends_at: input.ends_at,
                subject_id: input.subject_id,
            })
            .await?,
    ))
}

async fn remove_room(
    State(state): State<AppState>,
    Path(id): Path<RoomId>,
    Json(_): Json<serde_json::Value>,
) -> HttpResult {
    Ok(Json(state.recorder.execute(Action::RemoveRoom(id)).await?))
}

#[derive(Deserialize)]
struct SubjectAssignment {
    subject_id: Option<SubjectId>,
}
async fn assign_subject(
    State(state): State<AppState>,
    Path(id): Path<RecordingId>,
    Json(input): Json<SubjectAssignment>,
) -> HttpResult {
    Ok(Json(
        state
            .recorder
            .execute(Action::AssignSubject {
                id,
                subject_id: input.subject_id,
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
