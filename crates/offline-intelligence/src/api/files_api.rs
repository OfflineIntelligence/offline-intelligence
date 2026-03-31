
use axum::{
    extract::{Multipart, Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use tracing::{error, info};

use crate::shared_state::UnifiedAppState;
use crate::memory_db::{LocalFile, LocalFileTree};

#[derive(Debug, Serialize)]
pub struct FileEntryResponse {
    pub id: i64,
    pub name: String,
    pub path: String,
    #[serde(rename = "isDirectory")]
    pub is_directory: bool,
    pub size: i64,
    pub modified: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<FileEntryResponse>>,
}

impl From<LocalFile> for FileEntryResponse {
    fn from(f: LocalFile) -> Self {
        Self {
            id: f.id,
            name: f.name,
            path: f.path,
            is_directory: f.is_directory,
            size: f.size_bytes,
            modified: f.modified_at.to_rfc3339(),
            children: None,
        }
    }
}

impl From<LocalFileTree> for FileEntryResponse {
    fn from(t: LocalFileTree) -> Self {
        Self {
            id: t.file.id,
            name: t.file.name,
            path: t.file.path,
            is_directory: t.file.is_directory,
            size: t.file.size_bytes,
            modified: t.file.modified_at.to_rfc3339(),
            children: t.children.map(|c| c.into_iter().map(FileEntryResponse::from).collect()),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateFolderRequest {
    pub name: String,
    #[serde(default)]
    pub parent_id: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct DeleteQuery {
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub id: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q: String,
}

#[derive(Debug, Deserialize)]
pub struct UploadQuery {
    #[serde(default)]
    pub parent_id: Option<i64>,
}

pub async fn get_files(
    State(state): State<UnifiedAppState>,
) -> Result<impl IntoResponse, StatusCode> {
    let local_files = &state.shared_state.database_pool.local_files;
    
    match local_files.get_file_tree() {
        Ok(tree) => {
            let response: Vec<FileEntryResponse> = tree.into_iter()
                .map(FileEntryResponse::from)
                .collect();
            Ok(Json(response))
        }
        Err(e) => {
            error!("Failed to get files: {}", e);
            
            Ok(Json(Vec::<FileEntryResponse>::new()))
        }
    }
}

pub async fn get_file_by_id(
    State(state): State<UnifiedAppState>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, StatusCode> {
    let local_files = &state.shared_state.database_pool.local_files;
    
    match local_files.get_file(id) {
        Ok(file) => Ok(Json(FileEntryResponse::from(file))),
        Err(e) => {
            error!("Failed to get file {}: {}", id, e);
            Err(StatusCode::NOT_FOUND)
        }
    }
}

pub async fn get_file_content(
    State(state): State<UnifiedAppState>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, StatusCode> {
    let local_files = &state.shared_state.database_pool.local_files;
    
    match local_files.get_file_content_string(id) {
        Ok(content) => {
            Ok(Json(serde_json::json!({
                "id": id,
                "content": content
            })))
        }
        Err(e) => {
            error!("Failed to get file content {}: {}", id, e);
            Err(StatusCode::NOT_FOUND)
        }
    }
}

pub async fn search_files(
    State(state): State<UnifiedAppState>,
    Query(query): Query<SearchQuery>,
) -> Result<impl IntoResponse, StatusCode> {
    let local_files = &state.shared_state.database_pool.local_files;
    
    match local_files.search_files(&query.q) {
        Ok(files) => {
            let response: Vec<FileEntryResponse> = files.into_iter()
                .map(FileEntryResponse::from)
                .collect();
            Ok(Json(response))
        }
        Err(e) => {
            error!("Failed to search files: {}", e);
            Ok(Json(Vec::<FileEntryResponse>::new()))
        }
    }
}

pub async fn get_all_files(
    State(state): State<UnifiedAppState>,
) -> Result<impl IntoResponse, StatusCode> {
    let local_files = &state.shared_state.database_pool.local_files;
    
    match local_files.get_all_files() {
        Ok(files) => {
            let response: Vec<FileEntryResponse> = files.into_iter()
                .map(FileEntryResponse::from)
                .collect();
            Ok(Json(response))
        }
        Err(e) => {
            error!("Failed to get all files: {}", e);
            Ok(Json(Vec::<FileEntryResponse>::new()))
        }
    }
}

pub async fn create_folder(
    State(state): State<UnifiedAppState>,
    Json(request): Json<CreateFolderRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    let local_files = &state.shared_state.database_pool.local_files;
    
    if request.name.trim().is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }
    
    match local_files.create_folder(request.parent_id, &request.name) {
        Ok(folder) => {
            info!("Created folder: {}", folder.path);
            Ok(Json(serde_json::json!({
                "message": "Folder created successfully",
                "id": folder.id,
                "path": folder.path
            })))
        }
        Err(e) => {
            error!("Failed to create folder '{}': {}", request.name, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn upload_file(
    State(state): State<UnifiedAppState>,
    Query(query): Query<UploadQuery>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, StatusCode> {
    let local_files = &state.shared_state.database_pool.local_files;
    
    let mut file_count = 0;
    const MAX_FILES: usize = 16;
    const MAX_FILE_SIZE: usize = 10 * 1024 * 1024; 
    
    let allowed_extensions = [
        
        "pdf", "doc", "docx", "txt", "rtf", "odt",
        
        "xls", "xlsx", "csv", "ods",
        
        "ppt", "pptx", "odp",
        
        "js", "ts", "jsx", "tsx", "py", "java", "cpp", "c", "cs",
        "html", "css", "scss", "json", "xml", "yaml", "yml", "md",
        "go", "rs", "php", "rb", "swift", "kt", "scala", "sql",
        "sh", "bat", "ps1", "dockerfile", "env", "toml", "ini", "cfg"
    ];

    while let Some(field) = multipart.next_field().await.map_err(|e| {
        error!("Error reading multipart field: {}", e);
        StatusCode::BAD_REQUEST
    })? {
        if file_count >= MAX_FILES {
            return Err(StatusCode::BAD_REQUEST);
        }
        
        let file_name = field.file_name().unwrap_or("unknown_filename").to_string();
        
        let file_extension = std::path::Path::new(&file_name)
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("")
            .to_lowercase();
        
        if !allowed_extensions.contains(&file_extension.as_str()) {
            error!("File type not allowed: {}", file_extension);
            continue; 
        }
        
        let data = field.bytes().await.map_err(|e| {
            error!("Error reading file {}: {}", file_name, e);
            StatusCode::BAD_REQUEST
        })?;

        if data.len() > MAX_FILE_SIZE {
            error!("File {} exceeds size limit of {} bytes", file_name, MAX_FILE_SIZE);
            return Err(StatusCode::PAYLOAD_TOO_LARGE);
        }
        
        let mime_type = mime_guess::from_path(&file_name)
            .first()
            .map(|m| m.to_string());

        match local_files.upload_file(query.parent_id, &file_name, &data, mime_type.as_deref()) {
            Ok(file) => {
                info!("Uploaded file: {} ({} bytes)", file.path, data.len());
                file_count += 1;
            }
            Err(e) => {
                error!("Failed to upload file {}: {}", file_name, e);
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
        }
    }

    Ok(Json(serde_json::json!({
        "message": format!("Successfully uploaded {} file(s)", file_count),
        "count": file_count
    })))
}

pub async fn delete_file(
    State(state): State<UnifiedAppState>,
    Query(query): Query<DeleteQuery>,
) -> Result<impl IntoResponse, StatusCode> {
    let local_files = &state.shared_state.database_pool.local_files;
    
    let file_id = if let Some(id) = query.id {
        id
    } else if let Some(path) = &query.path {
        match local_files.get_file_by_path(path) {
            Ok(file) => file.id,
            Err(e) => {
                error!("File not found at path {}: {}", path, e);
                return Err(StatusCode::NOT_FOUND);
            }
        }
    } else {
        return Err(StatusCode::BAD_REQUEST);
    };
    
    match local_files.delete_file(file_id) {
        Ok(()) => {
            info!("Deleted file/folder with id {}", file_id);
            Ok(Json(serde_json::json!({
                "message": "File/directory deleted successfully"
            })))
        }
        Err(e) => {
            error!("Failed to delete file {}: {}", file_id, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn delete_file_by_id(
    State(state): State<UnifiedAppState>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, StatusCode> {
    let local_files = &state.shared_state.database_pool.local_files;
    
    match local_files.delete_file(id) {
        Ok(()) => {
            info!("Deleted file/folder with id {}", id);
            Ok(Json(serde_json::json!({
                "message": "File/directory deleted successfully"
            })))
        }
        Err(e) => {
            error!("Failed to delete file {}: {}", id, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn sync_files(
    State(state): State<UnifiedAppState>,
) -> Result<impl IntoResponse, StatusCode> {
    let local_files = &state.shared_state.database_pool.local_files;
    
    match local_files.sync_from_filesystem() {
        Ok(count) => {
            info!("Synced {} files from filesystem", count);
            Ok(Json(serde_json::json!({
                "message": format!("Synced {} files from filesystem", count),
                "count": count
            })))
        }
        Err(e) => {
            error!("Failed to sync files: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn resync_files(
    State(state): State<UnifiedAppState>,
) -> Result<impl IntoResponse, StatusCode> {
    let local_files = &state.shared_state.database_pool.local_files;
    
    match local_files.clear_all() {
        Ok(cleared) => {
            info!("Cleared {} entries from local_files", cleared);
        }
        Err(e) => {
            error!("Failed to clear local_files: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    }
    
    match local_files.sync_from_filesystem() {
        Ok(count) => {
            info!("Resynced {} files from filesystem", count);
            Ok(Json(serde_json::json!({
                "message": format!("Cleared and resynced {} files from filesystem", count),
                "count": count
            })))
        }
        Err(e) => {
            error!("Failed to sync files: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
