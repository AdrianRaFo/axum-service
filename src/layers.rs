use axum::body::Body;
use axum::http::{HeaderMap, HeaderName, HeaderValue, Request, Response, StatusCode};
use axum::response::IntoResponse;
use std::time::Duration;
use tower_http::request_id::{MakeRequestId, RequestId};
use tower_http::trace::{HttpMakeClassifier, TraceLayer};
use tracing::field::display;
use tracing::{info_span, Span};
use uuid::Uuid;

#[derive(Clone)]
pub struct WithRequestId;

const X_REQUEST_ID: HeaderName = HeaderName::from_static("x-request-id");

impl MakeRequestId for WithRequestId {
  fn make_request_id<B>(&mut self, request: &Request<B>) -> Option<RequestId> {
    let new_request_id = Uuid::new_v4().to_string();
    let request_id = match request.headers().get(X_REQUEST_ID) {
      Some(value) => value.to_str().unwrap(),
      None => new_request_id.as_str(),
    };
    Some(RequestId::new(HeaderValue::from_str(request_id).unwrap()))
  }
}

type MakeSpanTrace = fn(&Request<Body>) -> Span;
type OnRequestTrace = fn(&Request<Body>, &Span);
type OnResponseTrace = fn(&Response<Body>, Duration, &Span);

fn find_request_id(headers: &HeaderMap<HeaderValue>) -> &str {
  headers
    .get("x-request-id")
    .and_then(|value| value.to_str().ok())
    .unwrap_or("unknown")
}

pub fn trace_layer() -> TraceLayer<HttpMakeClassifier, MakeSpanTrace, OnRequestTrace, OnResponseTrace> {
  let make_span_trace: MakeSpanTrace = |request: &Request<Body>| {
    info_span!(
      "http_request",
      request_id = ?find_request_id(request.headers()),
      method = ?request.method(),
      path = request.uri().to_string(),
      status_code = tracing::field::Empty,
      latency = tracing::field::Empty
    )
  };

  let on_request_trace: OnRequestTrace = |request: &Request<Body>, _span: &Span| {
    tracing::info!(
      request_id = ?find_request_id(request.headers()),
      method = ?request.method(),
      path = ?request.uri(), "on_request"
    );
  };

  let on_response_trace: OnResponseTrace = |response: &Response<Body>, latency: Duration, span: &Span| {
    tracing::info!(
      request_id = ?find_request_id(response.headers()),
      status_code = ?response.status(),
      latency = ?latency, "on_response"
    );

    span
      .record("status_code", &display(response.status()))
      .record("latency", format!("{:?}", latency));
  };

  TraceLayer::new_for_http()
    .make_span_with(make_span_trace)
    .on_request(on_request_trace)
    .on_response(on_response_trace)
}

pub async fn not_found_layer() -> impl IntoResponse {
  (StatusCode::NOT_FOUND, "Not Found")
}
