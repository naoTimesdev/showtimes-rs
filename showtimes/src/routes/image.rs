use axum::{
    extract::{Path, State},
    response::IntoResponse,
};
use axum_extra::body::AsyncReadBody;
use serde::Deserialize;
use tokio::io::AsyncWriteExt;

use crate::state::{ShowtimesState, StorageShared};

#[derive(Deserialize)]
pub struct ImageQuery {
    id: String,
    filename: String,
    parent_id: Option<String>,
}

async fn common_reader(
    method: axum::http::Method,
    query: ImageQuery,
    fs_pool: StorageShared,
) -> impl IntoResponse {
    let (tx, rx) = tokio::io::duplex(65_536);
    futures::pin_mut!(tx);

    let body = AsyncReadBody::new(rx);

    let file = fs_pool
        .file_stat(
            &query.id,
            &query.filename,
            query.parent_id.as_deref(),
            Some(showtimes_fs::FsFileKind::Images),
        )
        .await;

    let raw_headers = match file {
        Ok(file) => {
            let raw_headers = vec![
                (axum::http::header::CONTENT_TYPE, file.content_type.clone()),
                (axum::http::header::CONTENT_LENGTH, file.size.to_string()),
                (
                    axum::http::header::CONTENT_DISPOSITION,
                    format!("inline; filename=\"{}\"", query.filename),
                ),
                (
                    axum::http::header::CACHE_CONTROL,
                    "public, max-age=604800, immutable".to_string(),
                ),
            ];

            if method == axum::http::Method::HEAD {
                let mut builder = axum::http::Response::builder();
                let headers = builder.headers_mut().unwrap();

                for (key, value) in raw_headers {
                    headers.insert(key, axum::http::HeaderValue::from_str(&value).unwrap());
                }

                return builder
                    .status(axum::http::StatusCode::OK)
                    .body(body)
                    .unwrap();
            }

            raw_headers
        }
        Err(e) => {
            tracing::error!("Failed to read file: {:?}", e);

            if method == axum::http::Method::HEAD {
                return axum::http::Response::builder()
                    .status(axum::http::StatusCode::NOT_FOUND)
                    .body(body)
                    .unwrap();
            }

            // Write something to tx so we can have same type
            tx.write(b"Not found").await.unwrap();

            // Return an error 404
            return axum::http::Response::builder()
                .status(axum::http::StatusCode::NOT_FOUND)
                .body(body)
                .unwrap();
        }
    };

    fs_pool
        .file_stream_download(
            &query.id,
            &query.filename,
            &mut tx,
            query.parent_id.as_deref(),
            Some(showtimes_fs::FsFileKind::Images),
        )
        .await
        .unwrap();

    let mut builder = axum::http::Response::builder();
    let headers = builder.headers_mut().unwrap();

    for (key, value) in raw_headers {
        headers.insert(key, axum::http::HeaderValue::from_str(&value).unwrap());
    }

    builder.body(body).unwrap()
}

pub async fn image_by_id(
    method: axum::http::Method,
    Path(query): Path<ImageQuery>,
    State(state): State<ShowtimesState>,
) -> impl IntoResponse {
    common_reader(method, query, state.storage).await
}
