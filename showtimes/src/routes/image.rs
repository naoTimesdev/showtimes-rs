use std::sync::Arc;

use axum::{
    extract::{Path, State},
    response::IntoResponse,
};
use axum_extra::body::AsyncReadBody;
use serde::Deserialize;
use tokio::io::AsyncWriteExt;

use crate::state::{SharedShowtimesState, StorageShared};

#[derive(Deserialize, Clone)]
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
    let (mut tx, rx) = tokio::io::duplex(65_536);

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
            tx.write_all(b"Not found").await.unwrap();

            // Return an error 404
            return axum::http::Response::builder()
                .status(axum::http::StatusCode::NOT_FOUND)
                .body(body)
                .unwrap();
        }
    };

    let q_clone = query.clone();
    tokio::spawn(async move {
        if let Err(e) = fs_pool
            .file_stream_download(
                &q_clone.id,
                &q_clone.filename,
                &mut tx,
                query.parent_id.as_deref(),
                Some(showtimes_fs::FsFileKind::Images),
            )
            .await
        {
            tracing::error!("Failed to read file: {:?}", e);
        }
    });

    let mut builder = axum::http::Response::builder();
    let headers = builder.headers_mut().unwrap();

    for (key, value) in raw_headers {
        headers.insert(key, axum::http::HeaderValue::from_str(&value).unwrap());
    }

    tracing::info!("Image sent: {:?}", query.filename);
    builder.body(body).unwrap()
}

pub async fn image_by_id(
    method: axum::http::Method,
    Path(query): Path<ImageQuery>,
    State(state): State<SharedShowtimesState>,
) -> impl IntoResponse {
    common_reader(method, query, Arc::clone(&state.storage)).await
}
