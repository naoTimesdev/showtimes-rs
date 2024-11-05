use std::sync::LazyLock;

use axum::{
    body::Body,
    extract::Request,
    http::{Method, StatusCode},
    response::Response,
};
use futures_util::future::BoxFuture;
use serde_json::json;
use showtimes_gql_common::MAX_IMAGE_SIZE;
use tower_layer::Layer;
use tower_service::Service;

const ESTIMATED_GRAPHQL_CONTENT: u64 = 2 * 1024 * 1024;
static GRAPHQL_ERROR: LazyLock<String> = LazyLock::new(|| {
    let json_data = json!({
        "data": null,
        "errors": [
            {
                "message": "Request entity is too large",
                "extensions": {
                    "code": "PAYLOAD_TOO_LARGE"
                }
            }
        ]
    });

    serde_json::to_string(&json_data).unwrap()
});
static GRAPHQL_ERROR_LIMIT: LazyLock<usize> = LazyLock::new(|| {
    (MAX_IMAGE_SIZE + ESTIMATED_GRAPHQL_CONTENT)
        .try_into()
        .expect("Failed to convert GraphQL limit to usize")
});

#[derive(Clone)]
pub struct GraphQLRequestLimit;

impl GraphQLRequestLimit {
    /// Create a new instance of the GraphQL request limit middleware
    pub fn new() -> Self {
        Self
    }
}

impl<S> Layer<S> for GraphQLRequestLimit {
    type Service = GraphQLRequestLimitMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        GraphQLRequestLimitMiddleware { inner }
    }
}

#[derive(Clone)]
pub struct GraphQLRequestLimitMiddleware<S> {
    inner: S,
}

impl<S> Service<Request> for GraphQLRequestLimitMiddleware<S>
where
    S: Service<Request, Response = Response> + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request) -> Self::Future {
        let headers = req.headers().clone();
        let req_method = req.method().clone();
        let future = self.inner.call(req);

        Box::pin(async move {
            // Check if this a multipart request
            if req_method == Method::POST
                && headers.contains_key("content-type")
                && headers["content-type"]
                    .to_str()
                    .unwrap_or_default()
                    .starts_with("multipart/form-data")
            {
                // Check if the content length is too large
                if let Some(content_length) = headers.get("content-length") {
                    let content_length = content_length
                        .to_str()
                        .unwrap_or_default()
                        .parse::<usize>()
                        .unwrap_or_default();

                    if content_length > *GRAPHQL_ERROR_LIMIT {
                        let body = Body::new(GRAPHQL_ERROR.clone());
                        let mut resp = Response::new(body);
                        *resp.status_mut() = StatusCode::PAYLOAD_TOO_LARGE;
                        return Ok(resp);
                    }
                }
            }

            future.await
        })
    }
}
