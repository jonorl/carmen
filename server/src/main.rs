//! The agency's records: each detective's arrests, so their rank follows
//! them from device to device. There are no accounts, as it's for family and
//! friends: a detective is just a name.
//!
//! GET /detectives lists everyone, most arrests first (the roster),
//! GET /detectives/{name} gives one detective's arrests (0 if they're new), and
//! PUT /detectives/{name} with {"arrests": n} records a tally. A tally never
//! goes down, so a device that was offline, or a request sent twice, can't
//! cost anyone an arrest.
//!
//! Settings come from the environment: CARMEN_DB_URL (Postgres), PORT_CARMEN
//! (default 8085) and FRONTEND_URL_CARMEN (the game's address, or several
//! separated by commas, for the browser's cross-site check).

use std::env;

use axum::extract::{Path, State};
use axum::http::{HeaderValue, Method, StatusCode, header};
use axum::routing::get;
use axum::{Json, Router};
use carmen_shared::{Detective, MAX_ARRESTS, MAX_NAME, Tally, key, tidy};
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use tower_http::cors::CorsLayer;

type Answer<T> = Result<T, (StatusCode, String)>;

#[tokio::main]
async fn main() {
    let db_url = env::var("CARMEN_DB_URL").expect("CARMEN_DB_URL must be set");
    let port: u16 = env::var("PORT_CARMEN").ok().and_then(|p| p.parse().ok()).unwrap_or(8085);
    let origins: Vec<HeaderValue> = env::var("FRONTEND_URL_CARMEN")
        .unwrap_or_default()
        .split(',')
        .filter_map(|o| o.trim().trim_end_matches('/').parse().ok())
        .collect();
    if origins.is_empty() {
        eprintln!("FRONTEND_URL_CARMEN isn't set, so browsers won't be allowed to call this server.");
    }

    let pool = PgPoolOptions::new().max_connections(5).connect(&db_url).await.expect("couldn't connect to the database");
    sqlx::migrate!().run(&pool).await.expect("couldn't bring the database up to date");

    let cors = CorsLayer::new().allow_origin(origins).allow_methods([Method::GET, Method::PUT]).allow_headers([header::CONTENT_TYPE]);
    let app = Router::new()
        .route("/health", get(health))
        .route("/detectives", get(roster))
        .route("/detectives/{name}", get(fetch).put(record))
        .layer(cors)
        .with_state(pool);

    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port)).await.expect("couldn't open the port");
    eprintln!("Listening on port {port}.");
    axum::serve(listener, app).with_graceful_shutdown(stopped()).await.expect("server failed");
}

/// Docker stops containers with SIGTERM; finish what's in hand first.
async fn stopped() {
    #[cfg(unix)]
    {
        let mut term = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()).expect("signal handler");
        tokio::select! {
            _ = term.recv() => {},
            _ = tokio::signal::ctrl_c() => {},
        }
    }
    #[cfg(not(unix))]
    let _ = tokio::signal::ctrl_c().await;
}

async fn health(State(pool): State<PgPool>) -> Answer<&'static str> {
    sqlx::query("SELECT 1").execute(&pool).await.map_err(trouble)?;
    Ok("ok")
}

/// More than the family will ever be, so a runaway table can't swamp the game.
const ROSTER_LIMIT: i64 = 500;

async fn roster(State(pool): State<PgPool>) -> Answer<Json<Vec<Detective>>> {
    let rows: Vec<(String, i32)> = sqlx::query_as("SELECT name, arrests FROM detectives ORDER BY arrests DESC, name_key LIMIT $1")
        .bind(ROSTER_LIMIT)
        .fetch_all(&pool)
        .await
        .map_err(trouble)?;
    Ok(Json(rows.into_iter().map(|(name, arrests)| Detective { name, arrests: arrests.max(0) as u32 }).collect()))
}

async fn fetch(State(pool): State<PgPool>, Path(name): Path<String>) -> Answer<Json<Detective>> {
    let name_key = checked(&name)?;
    let row: Option<(String, i32)> = sqlx::query_as("SELECT name, arrests FROM detectives WHERE name_key = $1")
        .bind(&name_key)
        .fetch_optional(&pool)
        .await
        .map_err(trouble)?;
    Ok(Json(match row {
        Some((name, arrests)) => Detective { name, arrests: arrests.max(0) as u32 },
        None => Detective { name: tidy(&name), arrests: 0 },
    }))
}

async fn record(State(pool): State<PgPool>, Path(name): Path<String>, Json(tally): Json<Tally>) -> Answer<Json<Detective>> {
    let name_key = checked(&name)?;
    if tally.arrests > MAX_ARRESTS {
        return Err((StatusCode::BAD_REQUEST, format!("No one has made more than {MAX_ARRESTS} arrests.")));
    }
    let (name, arrests): (String, i32) = sqlx::query_as(
        "INSERT INTO detectives (name_key, name, arrests) VALUES ($1, $2, $3)
         ON CONFLICT (name_key) DO UPDATE
             SET arrests = GREATEST(detectives.arrests, EXCLUDED.arrests),
                 name = EXCLUDED.name,
                 updated_at = now()
         RETURNING name, arrests",
    )
    .bind(&name_key)
    .bind(tidy(&name))
    .bind(tally.arrests as i32)
    .fetch_one(&pool)
    .await
    .map_err(trouble)?;
    Ok(Json(Detective { name, arrests: arrests.max(0) as u32 }))
}

fn checked(name: &str) -> Answer<String> {
    key(name).ok_or_else(|| (StatusCode::BAD_REQUEST, format!("A detective's name is 1 to {MAX_NAME} characters.")))
}

/// Log the details, but don't hand them to whoever asked.
fn trouble(e: sqlx::Error) -> (StatusCode, String) {
    eprintln!("Database error: {e}");
    (StatusCode::INTERNAL_SERVER_ERROR, "The records are unavailable.".to_string())
}
