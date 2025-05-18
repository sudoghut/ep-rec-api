//! Main API service for ep-rec-api
//! - Periodically pulls the eplot-data-compiler repo
//! - Serves two endpoints: series_with_year_month, get_content_by_series_id

use actix_web::{web, App, HttpServer, Responder, HttpResponse};
use serde::{Deserialize, Serialize};
use rusqlite::Connection;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use git2::Repository;

const REPO_URL: &str = "https://github.com/sudoghut/eplot-data-compiler.git";
const REPO_DIR: &str = "./eplot-data-compiler";
const DB_PATH: &str = "./eplot-data-compiler/data.db";
const GIT_PULL_INTERVAL_SECS: u64 = 60 * 60 * 24; // 24 hours

#[derive(Serialize)]
struct SeriesItem {
    id: i64,
    series_name: String,
}


#[derive(Deserialize)]
struct SeriesIdList {
    id_list: Vec<i64>,
}


async fn series_with_year_month(db_mutex: web::Data<Arc<Mutex<()>>>) -> impl Responder {
    let _lock = db_mutex.lock().await;
    let conn = match Connection::open(DB_PATH) {
        Ok(c) => c,
        Err(_) => return HttpResponse::InternalServerError().body("DB open error"),
    };
    let mut stmt = match conn.prepare(
        "SELECT id, series_name, series_year, series_month FROM series_data"
    ) {
        Ok(s) => s,
        Err(_) => return HttpResponse::InternalServerError().body("DB query error"),
    };
    let mut map: std::collections::BTreeMap<String, Vec<SeriesItem>> = std::collections::BTreeMap::new();
    let rows = match stmt.query_map([], |row| {
        let id: i64 = row.get(0)?;
        let name: String = row.get(1)?;
        let year: String = row.get(2)?;
        let month: String = row.get(3)?;
        Ok((id, name, year, month))
    }) {
        Ok(rows) => rows,
        Err(_) => return HttpResponse::InternalServerError().body("DB query error"),
    };
    for row in rows {
        if let Ok((id, name, year, month)) = row {
            let yyyymm = format!("{}{:0>2}", year, month);
            map.entry(yyyymm)
                .or_default()
                .push(SeriesItem { id, series_name: name });
        }
    }
    // Sort each list by series_name
    for v in map.values_mut() {
        v.sort_by(|a, b| a.series_name.cmp(&b.series_name));
    }
    HttpResponse::Ok().json(map)
}

async fn get_content_by_series_id(
    db_mutex: web::Data<Arc<Mutex<()>>>,
    payload: web::Json<SeriesIdList>,
) -> impl Responder {
    let _lock = db_mutex.lock().await;
    let conn = match Connection::open(DB_PATH) {
        Ok(c) => c,
        Err(_) => return HttpResponse::InternalServerError().body("DB open error"),
    };
    let result: std::collections::BTreeMap<String, Vec<String>> = std::collections::BTreeMap::new();
    // rusqlite doesn't support array binding directly, so build query dynamically
    let ids = &payload.id_list;
    if ids.is_empty() {
        return HttpResponse::Ok().json(result);
    }
    let mut sql = String::from("SELECT ep_name, ep_year, ep_month, ep_num, abstract FROM ep_data WHERE series_id IN (");
    sql += &ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    sql += ") ORDER BY ep_name, ep_year, ep_month, ep_num DESC";
    let mut stmt = match conn.prepare(&sql) {
        Ok(s) => s,
        Err(_) => return HttpResponse::InternalServerError().body("DB query error"),
    };
    let rows = match stmt.query_map(rusqlite::params_from_iter(ids.iter()), |row| {
        let ep_name: String = row.get(0)?;
        let ep_year: String = row.get(1)?;
        let ep_month: String = row.get(2)?;
        let ep_num: String = row.get(3)?;
        let abstract_: String = row.get(4)?;
        Ok((ep_name, ep_year, ep_month, ep_num, abstract_))
    }) {
        Ok(rows) => rows,
        Err(_) => return HttpResponse::InternalServerError().body("DB query error"),
    };
    // Group by ep_name, keep top 3 for each
    let mut group: std::collections::BTreeMap<String, Vec<(String, String, String, String)>> = std::collections::BTreeMap::new();
    let mut abstracts: std::collections::BTreeMap<String, Vec<String>> = std::collections::BTreeMap::new();
    for row in rows {
        if let Ok((ep_name, ep_year, ep_month, ep_num, abstract_)) = row {
            group.entry(ep_name.clone())
                .or_default()
                .push((ep_year, ep_month, ep_num, abstract_));
        }
    }
    for (ep_name, mut items) in group {
        // Sort by ep_year, ep_month, ep_num DESC
        items.sort_by(|a, b| {
            b.0.cmp(&a.0)
                .then(b.1.cmp(&a.1))
                .then(b.2.cmp(&a.2))
        });
        let abstracts_list = items.into_iter().take(3).map(|(_, _, _, abs)| abs).collect();
        abstracts.insert(ep_name, abstracts_list);
    }
    HttpResponse::Ok().json(abstracts)
}

async fn git_pull_task() {
    loop {
        // Clone if not exists
        if !Path::new(REPO_DIR).exists() {
            let _ = Repository::clone(REPO_URL, REPO_DIR);
        } else {
            if let Ok(repo) = Repository::open(REPO_DIR) {
                let mut remote = repo.find_remote("origin").unwrap();
                let _ = remote.fetch(&["main"], None, None);
                let refspec = repo.find_reference("refs/remotes/origin/main").unwrap();
                let oid = refspec.target().unwrap();
                let mut branch = repo.find_reference("refs/heads/main").unwrap();
                branch.set_target(oid, "Fast-forward").unwrap();
            }
        }
        tokio::time::sleep(Duration::from_secs(GIT_PULL_INTERVAL_SECS)).await;
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Start git pull background task
    tokio::spawn(git_pull_task());

    // Use a mutex to serialize DB access
    let db_mutex = Arc::new(Mutex::new(()));

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(db_mutex.clone()))
            .route("/series_with_year_month", web::get().to(series_with_year_month))
            .route("/series_with_year_month", web::post().to(series_with_year_month))
            .route("/get_content_by_series_id", web::post().to(get_content_by_series_id))
            .route("/get_content_by_series_id", web::get().to(get_content_by_series_id))
    })
    .bind(("127.0.0.1", 3001))?
    .run()
    .await
}
