mod layers;

use std::env;
use std::time::Duration;

use axum::response::Html;
use axum::routing::get;
use axum::{Router};
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::timeout::TimeoutLayer;
use tower_http::ServiceBuilderExt;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
  if env::var("RUST_LOG").is_err() {
    env::set_var("RUST_LOG", "info")
  }
  
  tracing_subscriber::registry()
    .with(
      tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "stream-service=debug,tower_http=debug,axum::rejection=trace".into()),
    )
    .with(tracing_subscriber::fmt::layer())
    .init();

  // build our application with a route

  let layers = ServiceBuilder::new()
    .set_x_request_id(layers::WithRequestId)
    .layer(layers::trace_layer())
    .layer(TimeoutLayer::new(Duration::from_secs(30)))
    .propagate_x_request_id();

  let app = Router::new()
    .route("/", get(handler))
    .fallback(layers::not_found_layer)
    .layer(layers);

  // run it
  let listener = TcpListener::bind("127.0.0.1:8080").await.unwrap();
  tracing::info!("listening on {}", listener.local_addr().unwrap());
  axum::serve(listener, app.into_make_service()).await.unwrap();
}

async fn handler() -> Html<&'static str> {
  Html("<h1>Hello, World!</h1>")
}
