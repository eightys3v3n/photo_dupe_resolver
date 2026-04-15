use axum::{
    extract::{Path, State},
    http::{header, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::database::Database;
use crate::shared_state::AppState;

pub struct AppStateManager {
    pub app_state: Arc<RwLock<AppState>>,
    pub db: Arc<Database>,
}

pub async fn run_server(state_manager: Arc<RwLock<AppState>>, db: Arc<Database>) -> anyhow::Result<()> {
    let app_state = AppStateManager {
        app_state: state_manager,
        db,
    };

    let app = Router::new()
        .route("/", get(index))
        .route("/api/status", get(get_status))
        .route("/api/images", get(get_images))
        .route("/api/start-scanner", post(start_scanner))
        .route("/api/stop-scanner", post(stop_scanner))
        .route("/api/start-hasher", post(start_hasher))
        .route("/api/stop-hasher", post(stop_hasher))
        .route("/api/start-grouper", post(start_grouper))
        .route("/api/stop-grouper", post(stop_grouper))
        .route("/api/duplicates", get(get_duplicates))
        .route("/api/image/:id", get(get_image_preview))
        .with_state(Arc::new(app_state));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    println!("Web UI running on http://{}", addr);

    axum::serve(listener, app).await?;
    Ok(())
}

async fn index() -> impl IntoResponse {
    let html = r#"
    <!DOCTYPE html>
    <html>
    <head>
        <title>Photo Dupe Resolver</title>
        <style>
            * { margin: 0; padding: 0; box-sizing: border-box; }
            body { font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; background: #f5f5f5; color: #24313f; }
            .container { max-width: 1280px; margin: 0 auto; padding: 20px; }
            header { background: #2c3e50; color: white; padding: 20px; border-radius: 10px; margin-bottom: 20px; }
            h1 { font-size: 32px; margin-bottom: 8px; }
            .section { background: white; padding: 20px; border-radius: 10px; margin-bottom: 20px; box-shadow: 0 2px 8px rgba(0,0,0,0.08); }
            h2 { color: #2c3e50; margin-bottom: 14px; font-size: 24px; }
            .tab-bar { display: flex; gap: 10px; flex-wrap: wrap; margin-bottom: 20px; }
            .tab-button { background: white; color: #2c3e50; border: 1px solid #d6dde5; padding: 10px 18px; border-radius: 999px; cursor: pointer; font-weight: 600; }
            .tab-button.active { background: #3498db; color: white; border-color: #3498db; }
            .tab-panel { display: none; }
            .tab-panel.active { display: block; }
            .status-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(160px, 1fr)); gap: 15px; }
            .status-item { background: #ecf0f1; padding: 15px; border-radius: 8px; }
            .status-label { font-size: 12px; color: #7f8c8d; text-transform: uppercase; margin-bottom: 6px; }
            .status-value { font-size: 24px; font-weight: bold; color: #2c3e50; }
            .controls { display: flex; gap: 10px; margin: 15px 0; flex-wrap: wrap; }
            button { background: #3498db; color: white; border: none; padding: 10px 18px; border-radius: 6px; cursor: pointer; }
            button:hover { background: #2980b9; }
            button.danger { background: #e74c3c; }
            button.danger:hover { background: #c0392b; }
            button:disabled { background: #bdc3c7; cursor: not-allowed; }
            .search-box { width: 100%; max-width: 460px; padding: 10px 12px; border: 1px solid #ccd3da; border-radius: 6px; margin-bottom: 12px; }
            .table-wrap { overflow-x: auto; }
            table { width: 100%; border-collapse: collapse; font-size: 14px; }
            th, td { padding: 10px 8px; border-bottom: 1px solid #e6eaee; text-align: left; vertical-align: top; }
            th { color: #2c3e50; background: #f8fafb; position: sticky; top: 0; }
            .status-pill { display: inline-block; padding: 4px 8px; border-radius: 999px; font-size: 12px; font-weight: 600; }
            .status-pill.hashed { background: #dff5e8; color: #1e7e46; }
            .status-pill.pending { background: #fff3cd; color: #8a6d1d; }
            .mono { font-family: Consolas, monospace; font-size: 12px; word-break: break-all; }
            .muted { color: #7f8c8d; }
            .success { color: #27ae60; }
            .error { color: #e74c3c; }
            .group-list { display: grid; gap: 16px; }
            .group-card { border: 1px solid #dfe6ec; border-radius: 10px; padding: 14px; background: #fbfcfd; }
            .group-header { display: flex; justify-content: space-between; gap: 12px; align-items: flex-start; margin-bottom: 12px; }
            .group-title { font-size: 18px; font-weight: 700; color: #2c3e50; }
            .group-meta { color: #5d6d7e; font-size: 13px; }
            .thumb-grid {
                display: flex;
                flex-wrap: nowrap;
                gap: 12px;
                overflow-x: auto;
                overflow-y: hidden;
                padding-bottom: 6px;
                scrollbar-width: thin;
            }
            .thumb-card {
                flex: 0 0 220px;
                min-width: 220px;
                background: white;
                border: 1px solid #e4e8ed;
                border-radius: 8px;
                padding: 10px;
            }
            .thumb-image-wrap {
                width: 100%;
                height: 180px;
                border-radius: 6px;
                background: #eef2f5;
                overflow: hidden;
                margin-bottom: 8px;
                display: flex;
                align-items: center;
                justify-content: center;
            }
            .thumb-image { width: 100%; height: 100%; object-fit: contain; display: block; }
            .thumb-fallback { width: 100%; height: 100%; display: flex; align-items: center; justify-content: center; color: #7f8c8d; font-size: 13px; }
            .thumb-name { font-weight: 600; margin-bottom: 4px; word-break: break-word; }
        </style>
    </head>
    <body>
        <div class="container">
            <header>
                <h1>📸 Photo Dupe Resolver</h1>
                <p>Find and manage duplicate photos in your collection</p>
            </header>

            <div class="tab-bar">
                <button class="tab-button active" data-tab="control" onclick="switchTab('control')">Control</button>
                <button class="tab-button" data-tab="images" onclick="switchTab('images')">Images</button>
                <button class="tab-button" data-tab="groups" onclick="switchTab('groups')">Groups</button>
            </div>

            <div id="tab-control" class="tab-panel active">
                <div class="section">
                    <h2>Status</h2>
                    <div class="status-grid">
                        <div class="status-item">
                            <div class="status-label">Images Discovered</div>
                            <div class="status-value" id="discovered">0</div>
                        </div>
                        <div class="status-item">
                            <div class="status-label">Images Hashed</div>
                            <div class="status-value" id="hashed">0</div>
                        </div>
                        <div class="status-item">
                            <div class="status-label">Hashing Speed</div>
                            <div class="status-value" id="speed">0/s</div>
                        </div>
                        <div class="status-item">
                            <div class="status-label">Duplicate Groups</div>
                            <div class="status-value" id="group-count">0</div>
                        </div>
                        <div class="status-item">
                            <div class="status-label">Scanner Status</div>
                            <div class="status-value" id="scanner-status">Idle</div>
                        </div>
                        <div class="status-item">
                            <div class="status-label">Hasher Status</div>
                            <div class="status-value" id="hasher-status">Idle</div>
                        </div>
                        <div class="status-item">
                            <div class="status-label">Grouper Status</div>
                            <div class="status-value" id="grouper-status">Idle</div>
                        </div>
                    </div>
                </div>

                <div class="section">
                    <h2>Scanner</h2>
                    <div class="controls">
                        <button onclick="startScanner()" id="start-scanner">Start Scanner</button>
                        <button class="danger" onclick="stopScanner()" id="stop-scanner" disabled>Stop Scanner</button>
                    </div>
                    <div id="scanner-message" class="muted">Use this to discover photo files.</div>
                </div>

                <div class="section">
                    <h2>Hasher</h2>
                    <div class="controls">
                        <button onclick="startHasher()" id="start-hasher">Start Hasher</button>
                        <button class="danger" onclick="stopHasher()" id="stop-hasher" disabled>Stop Hasher</button>
                    </div>
                    <div id="hasher-message" class="muted">Use this to compute content and perceptual hashes.</div>
                </div>

                <div class="section">
                    <h2>Grouper</h2>
                    <div class="controls">
                        <button onclick="startGrouper()" id="start-grouper">Run Grouper</button>
                        <button class="danger" onclick="stopGrouper()" id="stop-grouper" disabled>Stop Grouper</button>
                    </div>
                    <div id="grouper-message" class="muted">Build duplicate groups from hashed images.</div>
                </div>
            </div>

            <div id="tab-images" class="tab-panel">
                <div class="section">
                    <h2>Discovered Images</h2>
                    <input class="search-box" id="image-search" type="text" placeholder="Filter by path or status..." oninput="renderImages()" />
                    <div class="table-wrap">
                        <table>
                            <thead>
                                <tr>
                                    <th>Status</th>
                                    <th>Path</th>
                                    <th>Size</th>
                                    <th>Added</th>
                                    <th>Content Hash</th>
                                    <th>Perceptual Hash</th>
                                </tr>
                            </thead>
                            <tbody id="images-body">
                                <tr><td colspan="6" class="muted">No discovered images yet</td></tr>
                            </tbody>
                        </table>
                    </div>
                </div>
            </div>

            <div id="tab-groups" class="tab-panel">
                <div class="section">
                    <h2>Duplicate Groups</h2>
                    <input class="search-box" id="group-search" type="text" placeholder="Filter by hash or image path..." oninput="renderGroups()" />
                    <div id="groups-container" class="group-list">
                        <div class="muted">Run the grouper to populate duplicate groups.</div>
                    </div>
                </div>
            </div>
        </div>

        <script>
            let currentImages = [];
            let currentGroups = [];

            function escapeHtml(value) {
                return String(value ?? '')
                    .replaceAll('&', '&amp;')
                    .replaceAll('<', '&lt;')
                    .replaceAll('>', '&gt;')
                    .replaceAll('"', '&quot;')
                    .replaceAll("'", '&#39;');
            }

            function formatSize(bytes) {
                if (bytes < 1024) return bytes + ' B';
                if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + ' KB';
                if (bytes < 1024 * 1024 * 1024) return (bytes / (1024 * 1024)).toFixed(1) + ' MB';
                return (bytes / (1024 * 1024 * 1024)).toFixed(1) + ' GB';
            }

            function shortHash(hash) {
                return hash ? escapeHtml(hash.slice(0, 16)) + '…' : '<span class="muted">—</span>';
            }

            function basename(path) {
                const parts = String(path ?? '').split(/[/\\]/);
                return parts[parts.length - 1] || 'image';
            }

            function switchTab(tabName) {
                document.querySelectorAll('.tab-button').forEach(button => {
                    button.classList.toggle('active', button.dataset.tab === tabName);
                });
                document.querySelectorAll('.tab-panel').forEach(panel => {
                    panel.classList.toggle('active', panel.id === 'tab-' + tabName);
                });
            }

            function renderImages() {
                const query = document.getElementById('image-search').value.toLowerCase().trim();
                const rows = currentImages.filter(img => {
                    const haystack = [img.path, img.hash_status, img.created_at].join(' ').toLowerCase();
                    return haystack.includes(query);
                });

                const body = document.getElementById('images-body');
                if (!rows.length) {
                    body.innerHTML = '<tr><td colspan="6" class="muted">No matching images found</td></tr>';
                    return;
                }

                body.innerHTML = rows.map(img => `
                    <tr>
                        <td><span class="status-pill ${img.has_hash ? 'hashed' : 'pending'}">${escapeHtml(img.hash_status)}</span></td>
                        <td class="mono">${escapeHtml(img.path)}</td>
                        <td>${formatSize(img.size)}</td>
                        <td>${escapeHtml(img.created_at)}</td>
                        <td class="mono">${shortHash(img.content_hash)}</td>
                        <td class="mono">${shortHash(img.perceptual_hash)}</td>
                    </tr>
                `).join('');
            }

            function renderGroups() {
                const query = document.getElementById('group-search').value.toLowerCase().trim();
                const groups = currentGroups.filter(group => {
                    const imagePaths = (group.images || []).map(img => img.path).join(' ');
                    const haystack = [group.group_key, group.group_type, imagePaths].join(' ').toLowerCase();
                    return haystack.includes(query);
                });

                const container = document.getElementById('groups-container');
                if (!groups.length) {
                    container.innerHTML = '<div class="muted">No duplicate groups found.</div>';
                    return;
                }

                container.innerHTML = groups.map(group => `
                    <div class="group-card">
                        <div class="group-header">
                            <div>
                                <div class="group-title">${escapeHtml(group.group_type)}</div>
                                <div class="mono">${escapeHtml(group.group_key)}</div>
                            </div>
                            <div class="group-meta">${group.image_count} images</div>
                        </div>
                        <div class="thumb-grid">
                            ${(group.images || []).map(img => `
                                <div class="thumb-card">
                                    <div class="thumb-image-wrap">
                                        <img class="thumb-image" src="${escapeHtml(img.preview_url)}" alt="thumbnail" onerror="this.style.display='none'; this.nextElementSibling.style.display='flex';" />
                                        <div class="thumb-fallback" style="display:none;">🖼️ No Preview</div>
                                    </div>
                                    <div class="thumb-name">${escapeHtml(basename(img.path))}</div>
                                    <div class="mono muted">${escapeHtml(img.path)}</div>
                                    <div class="muted">${formatSize(img.size)}</div>
                                </div>
                            `).join('')}
                        </div>
                    </div>
                `).join('');
            }

            async function updateStatus() {
                try {
                    const response = await fetch('/api/status');
                    const data = await response.json();

                    document.getElementById('discovered').textContent = data.total_images_discovered;
                    document.getElementById('hashed').textContent = data.total_images_hashed;
                    document.getElementById('speed').textContent = data.hashing_speed.toFixed(2) + '/s';
                    document.getElementById('group-count').textContent = data.total_duplicate_groups;
                    document.getElementById('scanner-status').textContent = data.scanner_running ? 'Running' : 'Idle';
                    document.getElementById('hasher-status').textContent = data.hasher_running ? 'Running' : 'Idle';
                    document.getElementById('grouper-status').textContent = data.grouper_running ? 'Running' : 'Idle';

                    document.getElementById('start-scanner').disabled = data.scanner_running;
                    document.getElementById('stop-scanner').disabled = !data.scanner_running;
                    document.getElementById('start-hasher').disabled = data.hasher_running;
                    document.getElementById('stop-hasher').disabled = !data.hasher_running;
                    document.getElementById('start-grouper').disabled = data.grouper_running;
                    document.getElementById('stop-grouper').disabled = !data.grouper_running;
                } catch (e) {
                    console.error('Error fetching status:', e);
                }
            }

            async function updateImages() {
                try {
                    const response = await fetch('/api/images');
                    const data = await response.json();
                    currentImages = data.images || [];
                    renderImages();
                } catch (e) {
                    document.getElementById('images-body').innerHTML = '<tr><td colspan="6" class="error">Failed to load discovered images</td></tr>';
                }
            }

            async function updateGroups() {
                try {
                    const response = await fetch('/api/duplicates');
                    const data = await response.json();
                    currentGroups = data.groups || [];
                    renderGroups();
                } catch (e) {
                    document.getElementById('groups-container').innerHTML = '<div class="error">Failed to load duplicate groups</div>';
                }
            }

            async function startScanner() {
                try {
                    const response = await fetch('/api/start-scanner', { method: 'POST' });
                    if (response.ok) {
                        document.getElementById('scanner-message').innerHTML = '<span class="success">Scanner started</span>';
                        updateStatus();
                        updateImages();
                    }
                } catch (e) {
                    document.getElementById('scanner-message').innerHTML = '<span class="error">Error: ' + e.message + '</span>';
                }
            }

            async function stopScanner() {
                try {
                    const response = await fetch('/api/stop-scanner', { method: 'POST' });
                    if (response.ok) {
                        document.getElementById('scanner-message').innerHTML = '<span class="success">Scanner stopped</span>';
                        updateStatus();
                    }
                } catch (e) {
                    document.getElementById('scanner-message').innerHTML = '<span class="error">Error: ' + e.message + '</span>';
                }
            }

            async function startHasher() {
                try {
                    const response = await fetch('/api/start-hasher', { method: 'POST' });
                    if (response.ok) {
                        document.getElementById('hasher-message').innerHTML = '<span class="success">Hasher started</span>';
                        updateStatus();
                        updateImages();
                        updateGroups();
                    }
                } catch (e) {
                    document.getElementById('hasher-message').innerHTML = '<span class="error">Error: ' + e.message + '</span>';
                }
            }

            async function stopHasher() {
                try {
                    const response = await fetch('/api/stop-hasher', { method: 'POST' });
                    if (response.ok) {
                        document.getElementById('hasher-message').innerHTML = '<span class="success">Hasher stopped</span>';
                        updateStatus();
                    }
                } catch (e) {
                    document.getElementById('hasher-message').innerHTML = '<span class="error">Error: ' + e.message + '</span>';
                }
            }

            async function startGrouper() {
                try {
                    const response = await fetch('/api/start-grouper', { method: 'POST' });
                    if (response.ok) {
                        document.getElementById('grouper-message').innerHTML = '<span class="success">Grouper completed</span>';
                        updateStatus();
                        updateGroups();
                    }
                } catch (e) {
                    document.getElementById('grouper-message').innerHTML = '<span class="error">Error: ' + e.message + '</span>';
                }
            }

            async function stopGrouper() {
                try {
                    const response = await fetch('/api/stop-grouper', { method: 'POST' });
                    if (response.ok) {
                        document.getElementById('grouper-message').innerHTML = '<span class="success">Grouper stopped</span>';
                        updateStatus();
                    }
                } catch (e) {
                    document.getElementById('grouper-message').innerHTML = '<span class="error">Error: ' + e.message + '</span>';
                }
            }

            setInterval(() => {
                updateStatus();
                updateImages();
                updateGroups();
            }, 1500);

            switchTab('control');
            updateStatus();
            updateImages();
            updateGroups();
        </script>
    </body>
    </html>
    "#;
    (StatusCode::OK, Html(html))
}

struct Html<T>(T);

impl<T> IntoResponse for Html<T>
where
    T: Into<String>,
{
    fn into_response(self) -> axum::response::Response {
        (
            [(axum::http::header::CONTENT_TYPE, "text/html; charset=utf-8")],
            self.0.into(),
        )
            .into_response()
    }
}

impl<T> std::fmt::Debug for Html<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Html").finish()
    }
}

#[derive(serde::Serialize)]
struct ApiStatus {
    total_images_discovered: usize,
    total_images_hashed: usize,
    total_images_to_hash: usize,
    total_duplicate_groups: usize,
    hashing_speed: f64,
    scanner_running: bool,
    hasher_running: bool,
    grouper_running: bool,
}

#[derive(serde::Serialize)]
struct ImageListResponse {
    images: Vec<ImageRow>,
}

#[derive(serde::Serialize)]
struct ImageRow {
    id: String,
    path: String,
    size: u64,
    created_at: String,
    content_hash: String,
    perceptual_hash: String,
    has_hash: bool,
    hash_status: String,
}

#[derive(serde::Serialize, Clone)]
struct GroupImageRow {
    id: String,
    path: String,
    size: u64,
    content_hash: String,
    perceptual_hash: String,
    preview_url: String,
}

#[derive(serde::Serialize, Clone)]
struct DuplicateGroupRow {
    group_key: String,
    group_type: String,
    image_count: usize,
    images: Vec<GroupImageRow>,
}

#[derive(serde::Serialize)]
struct DuplicateGroupResponse {
    groups: Vec<DuplicateGroupRow>,
}

async fn get_status(State(state): State<Arc<AppStateManager>>) -> Json<ApiStatus> {
    let app_state = state.app_state.read().await;
    Json(ApiStatus {
        total_images_discovered: app_state.total_images_discovered,
        total_images_hashed: app_state.total_images_hashed,
        total_images_to_hash: app_state.total_images_to_hash,
        total_duplicate_groups: app_state.total_duplicate_groups,
        hashing_speed: app_state.hashing_speed,
        scanner_running: app_state.scanner_running,
        hasher_running: app_state.hasher_running,
        grouper_running: app_state.grouper_running,
    })
}

async fn get_images(
    State(state): State<Arc<AppStateManager>>,
) -> std::result::Result<Json<ImageListResponse>, StatusCode> {
    let images = state.db.get_all_images().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let rows = images
        .into_iter()
        .map(|image| {
            let has_hash = image.content_hash.is_some();
            ImageRow {
                id: image.id,
                path: image.path,
                size: image.size,
                created_at: image.created_at,
                content_hash: image.content_hash.unwrap_or_default(),
                perceptual_hash: image.perceptual_hash.unwrap_or_default(),
                has_hash,
                hash_status: if has_hash { "Hashed".to_string() } else { "Pending".to_string() },
            }
        })
        .collect();

    Ok(Json(ImageListResponse { images: rows }))
}

fn build_duplicate_group_rows(
    groups: Vec<(String, Vec<crate::database::Image>)>,
) -> (
    Vec<DuplicateGroupRow>,
    Vec<crate::shared_state::DuplicateGroup>,
) {
    let mut response_groups = Vec::new();
    let mut state_groups = Vec::new();

    for (hash, mut images) in groups {
        images.sort_by(|left, right| left.path.cmp(&right.path));

        let perceptual_hash = images
            .iter()
            .find_map(|image| image.perceptual_hash.clone())
            .unwrap_or_default();

        let response_images = images
            .iter()
            .map(|image| GroupImageRow {
                id: image.id.clone(),
                path: image.path.clone(),
                size: image.size,
                content_hash: image.content_hash.clone().unwrap_or_default(),
                perceptual_hash: image.perceptual_hash.clone().unwrap_or_default(),
                preview_url: format!("/api/image/{}", image.id),
            })
            .collect::<Vec<_>>();

        state_groups.push(crate::shared_state::DuplicateGroup {
            hash: hash.clone(),
            perceptual_hash,
            images: images
                .iter()
                .map(|image| crate::shared_state::ImageInfo {
                    id: image.id.clone(),
                    path: image.path.clone(),
                    size: image.size,
                    content_hash: image.content_hash.clone().unwrap_or_default(),
                    perceptual_hash: image.perceptual_hash.clone().unwrap_or_default(),
                })
                .collect(),
        });

        response_groups.push(DuplicateGroupRow {
            group_key: hash,
            group_type: "Exact Match".to_string(),
            image_count: response_images.len(),
            images: response_images,
        });
    }

    response_groups.sort_by(|left, right| {
        right
            .image_count
            .cmp(&left.image_count)
            .then_with(|| left.group_key.cmp(&right.group_key))
    });

    (response_groups, state_groups)
}

async fn refresh_duplicate_groups(
    state: &Arc<AppStateManager>,
) -> std::result::Result<Vec<DuplicateGroupRow>, StatusCode> {
    let groups = state
        .db
        .get_duplicate_groups()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let (response_groups, state_groups) = build_duplicate_group_rows(groups);

    let mut app_state = state.app_state.write().await;
    app_state.total_duplicate_groups = response_groups.len();
    app_state.duplicate_groups = state_groups;
    app_state.grouper_running = false;

    Ok(response_groups)
}

async fn start_scanner(State(state): State<Arc<AppStateManager>>) -> StatusCode {
    let mut app_state = state.app_state.write().await;
    app_state.scanner_running = true;
    StatusCode::OK
}

async fn stop_scanner(State(state): State<Arc<AppStateManager>>) -> StatusCode {
    let mut app_state = state.app_state.write().await;
    app_state.scanner_running = false;
    StatusCode::OK
}

async fn start_hasher(State(state): State<Arc<AppStateManager>>) -> StatusCode {
    let mut app_state = state.app_state.write().await;
    app_state.hasher_running = true;
    StatusCode::OK
}

async fn stop_hasher(State(state): State<Arc<AppStateManager>>) -> StatusCode {
    let mut app_state = state.app_state.write().await;
    app_state.hasher_running = false;
    StatusCode::OK
}

async fn start_grouper(State(state): State<Arc<AppStateManager>>) -> StatusCode {
    {
        let mut app_state = state.app_state.write().await;
        app_state.grouper_running = true;
    }

    match refresh_duplicate_groups(&state).await {
        Ok(_) => StatusCode::OK,
        Err(status) => status,
    }
}

async fn stop_grouper(State(state): State<Arc<AppStateManager>>) -> StatusCode {
    let mut app_state = state.app_state.write().await;
    app_state.grouper_running = false;
    StatusCode::OK
}

async fn get_duplicates(State(state): State<Arc<AppStateManager>>) -> Json<DuplicateGroupResponse> {
    match refresh_duplicate_groups(&state).await {
        Ok(groups) => Json(DuplicateGroupResponse { groups }),
        Err(_) => Json(DuplicateGroupResponse { groups: vec![] }),
    }
}

fn image_content_type(path: &str) -> &'static str {
    let extension = path.rsplit('.').next().unwrap_or_default().to_ascii_lowercase();
    match extension.as_str() {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "bmp" => "image/bmp",
        "tiff" | "tif" => "image/tiff",
        _ => "application/octet-stream",
    }
}

async fn get_image_preview(
    Path(id): Path<String>,
    State(state): State<Arc<AppStateManager>>,
) -> impl IntoResponse {
    match state.db.get_image_by_id(&id) {
        Ok(Some(image)) => match std::fs::read(&image.path) {
            Ok(bytes) => (
                [(header::CONTENT_TYPE, image_content_type(&image.path))],
                bytes,
            )
                .into_response(),
            Err(_) => StatusCode::NOT_FOUND.into_response(),
        },
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Image;

    #[test]
    fn test_app_state_manager_creation() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db = Arc::new(Database::new(db_path.to_str().unwrap()).unwrap());
        let state = Arc::new(RwLock::new(AppState::new()));
        let manager = AppStateManager {
            app_state: state.clone(),
            db,
        };
        assert!(!manager.app_state.blocking_read().scanner_running);
    }

    #[tokio::test]
    async fn test_index_returns_ok_html() {
        let response = index().await.into_response();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_index_contains_expected_tabs() {
        let response = index().await.into_response();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let html = String::from_utf8(body.to_vec()).unwrap();

        assert!(html.contains("Control"));
        assert!(html.contains("Images"));
        assert!(html.contains("Groups"));
        assert!(html.contains("Grouper"));
    }

    #[tokio::test]
    async fn test_index_contains_scrollable_group_preview_styles() {
        let response = index().await.into_response();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let html = String::from_utf8(body.to_vec()).unwrap();

        assert!(html.contains("overflow-x: auto"));
        assert!(html.contains("object-fit: contain"));
        assert!(html.contains("flex-wrap: nowrap"));
    }

    #[tokio::test]
    async fn test_status_reflects_state() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db = Arc::new(Database::new(db_path.to_str().unwrap()).unwrap());
        let state = Arc::new(RwLock::new(AppState::new()));
        {
            let mut guard = state.write().await;
            guard.total_images_discovered = 12;
            guard.total_images_hashed = 7;
            guard.scanner_running = true;
        }

        let manager = Arc::new(AppStateManager { app_state: state, db });
        let Json(status) = get_status(State(manager)).await;

        assert_eq!(status.total_images_discovered, 12);
        assert_eq!(status.total_images_hashed, 7);
        assert!(status.scanner_running);
        assert!(!status.hasher_running);
    }

    #[tokio::test]
    async fn test_images_endpoint_returns_hash_status() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db = Arc::new(Database::new(db_path.to_str().unwrap()).unwrap());
        let state = Arc::new(RwLock::new(AppState::new()));

        db.insert_images_batch(&[
            Image {
                id: "1".to_string(),
                path: "/photos/a.jpg".to_string(),
                size: 1234,
                content_hash: None,
                perceptual_hash: None,
                created_at: "2026-04-14T00:00:00Z".to_string(),
            },
            Image {
                id: "2".to_string(),
                path: "/photos/b.jpg".to_string(),
                size: 4321,
                content_hash: Some("abc123".to_string()),
                perceptual_hash: Some("def456".to_string()),
                created_at: "2026-04-14T00:00:00Z".to_string(),
            },
        ]).unwrap();

        let manager = Arc::new(AppStateManager { app_state: state, db });
        let Json(response) = get_images(State(manager)).await.unwrap();

        assert_eq!(response.images.len(), 2);
        assert!(response.images.iter().any(|img| img.hash_status == "Pending"));
        assert!(response.images.iter().any(|img| img.hash_status == "Hashed"));
    }

    #[tokio::test]
    async fn test_duplicates_endpoint_returns_grouped_images() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db = Arc::new(Database::new(db_path.to_str().unwrap()).unwrap());
        let state = Arc::new(RwLock::new(AppState::new()));

        db.insert_images_batch(&[
            Image {
                id: "1".to_string(),
                path: "/photos/a.jpg".to_string(),
                size: 1234,
                content_hash: Some("same-hash".to_string()),
                perceptual_hash: Some("same-phash".to_string()),
                created_at: "2026-04-14T00:00:00Z".to_string(),
            },
            Image {
                id: "2".to_string(),
                path: "/photos/b.jpg".to_string(),
                size: 4321,
                content_hash: Some("same-hash".to_string()),
                perceptual_hash: Some("same-phash".to_string()),
                created_at: "2026-04-14T00:00:01Z".to_string(),
            },
        ]).unwrap();

        let manager = Arc::new(AppStateManager { app_state: state, db });
        let Json(response) = get_duplicates(State(manager)).await;

        assert_eq!(response.groups.len(), 1);
        assert_eq!(response.groups[0].image_count, 2);
        assert_eq!(response.groups[0].group_type, "Exact Match");
    }
}
