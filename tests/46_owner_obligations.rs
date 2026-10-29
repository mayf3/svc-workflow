//! Accepted OWNER-OBL contracts on unique retained isolated PostgreSQL fixtures.
#[allow(unused_imports)]
#[path = "common/mod.rs"]
mod common;
use axum::{
    body::{to_bytes, Body},
    http::Request,
};
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;
#[path = "owner_obligations/http_support.rs"]
mod http_support;

#[path = "owner_obligations/support.rs"]
mod support;
use support::*;

#[path = "owner_obligations/migration.rs"]
mod migration;

#[path = "owner_obligations/publication.rs"]
mod publication;

#[path = "owner_obligations/intents.rs"]
mod intents;

#[path = "owner_obligations/effects.rs"]
mod effects;

#[path = "owner_obligations/http.rs"]
mod http;

#[path = "owner_obligations/review.rs"]
mod review;

#[path = "owner_obligations/stops.rs"]
mod stops;

#[path = "owner_obligations/case_projection.rs"]
mod case_projection;
#[path = "owner_obligations/context.rs"]
mod context;
#[path = "owner_obligations/context_support.rs"]
mod context_support;
