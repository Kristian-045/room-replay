use cesnet_dvr::{
    http::{self, AppState},
    library::Library,
    recorder::{Action, Recorder},
};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();
    let data = PathBuf::from(std::env::var("DVR_DATA_DIR").unwrap_or_else(|_| "data".into()));
    let frontend =
        PathBuf::from(std::env::var("DVR_WEB_DIR").unwrap_or_else(|_| "web/dist".into()));
    let bind = std::env::var("DVR_BIND").unwrap_or_else(|_| "127.0.0.1:3000".into());
    let listener = tokio::net::TcpListener::bind(&bind).await?;
    let library = Library::open(&data)?;
    let media = library.media.clone();
    let ffmpeg = std::env::var("DVR_FFMPEG").unwrap_or_else(|_| "ffmpeg".into());
    let (recorder, worker) = Recorder::spawn(library, ffmpeg.clone());
    let app = http::router(
        AppState {
            recorder: recorder.clone(),
            ffmpeg,
            allow_test_sources: std::env::var("DVR_ALLOW_TEST_SOURCES").as_deref() == Ok("1"),
        },
        media,
        frontend,
    );
    tracing::info!(%bind, "DVR listening");
    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let mut terminate =
                tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                    .expect("signal handler");
            tokio::select! { _ = tokio::signal::ctrl_c() => {}, _ = terminate.recv() => {} }
        })
        .await?;
    recorder.execute(Action::Shutdown).await?;
    worker.await??;
    Ok(())
}
