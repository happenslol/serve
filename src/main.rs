use anyhow::{anyhow, bail, Result};
use axum::{
  extract::Request,
  handler::HandlerWithoutStateExt,
  http::StatusCode,
  middleware::{self, Next},
  response::Html,
  Router,
};
use clap::Parser;
use colored::Colorize;
use std::{net::SocketAddr, path::PathBuf};
use tokio::fs;
use tower_http::services::ServeDir;
use tracing::info;

#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
  /// The port to listen on
  #[arg(short, long, default_value_t = 5000)]
  port: u16,

  /// The address to listen on
  #[arg(short, long, default_value_t = String::from("127.0.0.1"))]
  bind: String,

  /// Whether to open the browser
  #[arg(short, long, default_value_t = true)]
  open: bool,

  /// The directory to serve
  #[arg(default_value_t = String::from("."))]
  path: String,
}

#[tokio::main]
async fn main() -> Result<()> {
  tracing_subscriber::fmt::init();

  let args = Args::parse();
  let (path, file) = get_path(args.path).await?;

  let serve_dir = ServeDir::new(&path)
    .append_index_html_on_directories(true)
    .not_found_service(not_found.into_service());

  let app = Router::new()
    .fallback_service(serve_dir)
    .layer(middleware::from_fn(log));

  let addr = format!("{}:{}", args.bind, args.port).parse::<SocketAddr>()?;
  let listener = tokio::net::TcpListener::bind(addr).await?;

  let handle = tokio::spawn(async move {
    info!(
      "Serving {} on {}",
      path.to_string_lossy().blue(),
      addr.to_string().green()
    );
    axum::serve(listener, app).await
  });

  if args.open {
    let url = format!(
      "http://{}:{}{}",
      args.bind,
      args.port,
      file.map_or_else(String::new, |f| format!("/{}", f))
    );

    open::that_detached(&url)?;
  }

  Ok(handle.await??)
}

async fn get_path(path: String) -> Result<(PathBuf, Option<String>)> {
  let path = PathBuf::from(path);

  let meta = fs::metadata(&path).await?;
  if meta.is_dir() {
    Ok((path, None))
  } else if meta.is_file() {
    let parent = path
      .parent()
      .ok_or_else(|| anyhow!("Failed to get parent"))?;

    let file = path
      .file_name()
      .ok_or_else(|| anyhow!("Failed to get filename"))?;

    Ok((
      parent.to_path_buf(),
      Some(file.to_string_lossy().to_string()),
    ))
  } else {
    bail!("Path must either be a file or a directory")
  }
}

const NOT_FOUND: &str = include_str!("not-found.html");

async fn not_found() -> Html<&'static str> {
  Html(NOT_FOUND)
}

async fn log(req: Request, next: Next) -> Result<axum::response::Response, StatusCode> {
  let method = req.method().to_string();
  let uri = req.uri().to_string();

  let res = next.run(req).await;
  let status = res.status().as_u16().to_string();

  info!("{} {} {}", status.cyan(), method.bold().yellow(), uri);

  Ok(res)
}
