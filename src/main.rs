mod config;
mod cli;
mod database;
mod scanner;
mod hasher;
mod web_ui;
mod shared_state;

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing_subscriber;
use database::Database;
use scanner::Scanner;
use hasher::Hasher;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    // Parse CLI arguments
    let args = cli::parse_args();

    // Load configuration
    let mut config = config::Config::load(&args.config)?;
    config.apply_cli_overrides(&args);

    // Initialize shared state
    let state = Arc::new(RwLock::new(shared_state::AppState::new()));

    // Initialize database
    let db = Arc::new(Database::new(&config.db_path)?);

    // Start web UI in a separate task
    let state_clone = Arc::clone(&state);
    let db_for_web = Arc::clone(&db);
    tokio::spawn(async move {
        if let Err(e) = web_ui::run_server(state_clone, db_for_web).await {
            eprintln!("Web UI error: {}", e);
        }
    });

    tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
    let _ = webbrowser::open("http://127.0.0.1:3000");

    println!("Opening Web UI at http://127.0.0.1:3000");
    println!("Application initialized. Scanner and hasher are ready to start.");

    // Scanner task
    let scanner = Scanner::new(
        db.clone(),
        state.clone(),
        config.scanner_batch_size,
    );

    let scanner_state = state.clone();
    let scanner_paths = config.scan_paths.clone();

    let _scanner_task = tokio::spawn(async move {
        loop {
            let should_run = {
                let s = scanner_state.read().await;
                s.scanner_running && !scanner_paths.is_empty()
            };

            if should_run {
                if let Err(e) = scanner.start(&scanner_paths).await {
                    eprintln!("Scanner error: {}", e);
                }
                let mut s = scanner_state.write().await;
                s.scanner_running = false;
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
    });

    // Hasher task
    let hasher = Hasher::new(
        db.clone(),
        state.clone(),
        config.hasher_worker_threads,
        config.hasher_batch_queue_size,
        config.hasher_batch_size,
    );

    let hasher_state = state.clone();
    let _hasher_task = tokio::spawn(async move {
        loop {
            let should_run = hasher_state.read().await.hasher_running;

            if should_run {
                if let Err(e) = hasher.start().await {
                    eprintln!("Hasher error: {}", e);
                }
                let mut s = hasher_state.write().await;
                s.hasher_running = false;
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
    });

    // Keep the application running indefinitely
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
    }
}
