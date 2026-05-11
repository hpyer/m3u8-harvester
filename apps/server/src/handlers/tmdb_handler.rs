use crate::AppState;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use m3u8_core::{TmdbSearchResult, TmdbSeasonDetails};
use serde::Deserialize;
use std::sync::Arc;

#[derive(Debug, Deserialize)]
pub struct SearchTmdbQuery {
    pub query: String,
}

pub async fn search_tmdb(
    State(state): State<Arc<AppState>>,
    Query(query): Query<SearchTmdbQuery>,
) -> Result<Json<Vec<TmdbSearchResult>>, (StatusCode, String)> {
    state
        .tmdb_service
        .search(&query.query)
        .await
        .map(Json)
        .map_err(|err| (StatusCode::BAD_REQUEST, err.to_string()))
}

pub async fn get_tmdb_tv_season(
    State(state): State<Arc<AppState>>,
    Path((series_id, season_number)): Path<(i64, i32)>,
) -> Result<Json<TmdbSeasonDetails>, (StatusCode, String)> {
    state
        .tmdb_service
        .tv_season(series_id, season_number)
        .await
        .map(Json)
        .map_err(|err| (StatusCode::BAD_REQUEST, err.to_string()))
}
