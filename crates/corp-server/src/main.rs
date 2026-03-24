use clap::Parser;

use corp_server::routes::router;
use corp_server::state::AppState;

#[derive(Parser)]
#[command(name = "corp-server", about = "Corporate governance server", version)]
enum Cmd {
    /// Start the HTTP server (default)
    Serve,

    /// Execute a single API request in-process and print the JSON response.
    ///
    /// This allows the CLI to operate on a local git repo without a running
    /// server — just shell out to `corp-server call GET /v1/entities`.
    Call {
        /// HTTP method (GET, POST, PUT, PATCH, DELETE)
        method: String,
        /// Request path (e.g. /v1/entities)
        path: String,
        /// JSON request body (omit for GET/DELETE, or pass "-" to read stdin)
        body: Option<String>,
    },
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "corp_server=warn".parse().unwrap()),
        )
        .init();

    let cmd = Cmd::try_parse().unwrap_or(Cmd::Serve);

    match cmd {
        Cmd::Serve => run_server().await,
        Cmd::Call { method, path, body } => run_call(method, path, body).await,
    }
}

async fn run_server() {
    // Re-init tracing at info level for the server.
    let state = AppState::from_env().await;
    let app = router(state);

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], 8000));
    tracing::info!(%addr, "listening");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

/// Execute a single request against the in-process router and print the
/// response body to stdout.  Exit code 0 on 2xx, 1 otherwise.
async fn run_call(method: String, path: String, body: Option<String>) {
    use axum::body::Body;
    use axum::http::{Method, Request};
    use tower::ServiceExt;

    let state = AppState::from_env().await;
    let app = router(state);

    // Parse method.
    let method: Method = method.to_uppercase().parse().unwrap_or_else(|_| {
        eprintln!("invalid HTTP method: {method}");
        std::process::exit(2);
    });

    // Read body: from arg, from stdin, or empty.
    let body_bytes = match body.as_deref() {
        Some("-") => {
            use std::io::Read;
            let mut buf = String::new();
            std::io::stdin().read_to_string(&mut buf).unwrap_or(0);
            buf.into_bytes()
        }
        Some(b) => b.as_bytes().to_vec(),
        None => vec![],
    };

    // Build the auth header from env.
    let jwt_or_key = std::env::var("CORP_API_KEY")
        .or_else(|_| std::env::var("CORP_JWT_TOKEN"))
        .unwrap_or_default();

    let mut builder = Request::builder().method(method).uri(&path);

    if !body_bytes.is_empty() {
        builder = builder.header("content-type", "application/json");
    }
    if !jwt_or_key.is_empty() {
        builder = builder.header("authorization", format!("Bearer {jwt_or_key}"));
    }

    let req = builder.body(Body::from(body_bytes)).unwrap_or_else(|e| {
        eprintln!("bad request: {e}");
        std::process::exit(2);
    });

    let resp = app.oneshot(req).await.unwrap_or_else(|e| {
        eprintln!("router error: {e}");
        std::process::exit(1);
    });

    let status = resp.status();

    // Read the response body.
    let body_bytes = axum::body::to_bytes(resp.into_body(), 10 * 1024 * 1024)
        .await
        .unwrap_or_default();

    // Print the body to stdout.
    if !body_bytes.is_empty() {
        let _ = std::io::Write::write_all(&mut std::io::stdout(), &body_bytes);
        // Ensure trailing newline.
        if body_bytes.last() != Some(&b'\n') {
            println!();
        }
    }

    if !status.is_success() {
        std::process::exit(1);
    }
}
