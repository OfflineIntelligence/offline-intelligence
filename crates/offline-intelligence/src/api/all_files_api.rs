//! All Files API endpoints - unlimited storage for all file formats
//!
//! Provides user-managed file storage for context inclusion via @filename.

use axum::{
    extract::{Multipart, Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use tracing::{error, info, warn};

use crate::shared_state::UnifiedAppState;
use crate::memory_db::{AllFile, AllFileTree};

/// Response structure for all file entries
#[derive(Debug, Serialize)]
pub struct AllFileEntryResponse {
    pub id: i64,
    pub name: String,
    pub path: String,
    #[serde(rename = "isDirectory")]
    pub is_directory: bool,
    pub size: i64,
    pub modified: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_accessed: Option<String>,
    pub access_count: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<AllFileEntryResponse>>,
}

impl From<AllFile> for AllFileEntryResponse {
    fn from(f: AllFile) -> Self {
        Self {
            id: f.id,
            name: f.name,
            path: f.path,
            is_directory: f.is_directory,
            size: f.size_bytes,
            modified: f.modified_at.to_rfc3339(),
            last_accessed: f.last_accessed.map(|dt| dt.to_rfc3339()),
            access_count: f.access_count,
            children: None,
        }
    }
}

impl From<AllFileTree> for AllFileEntryResponse {
    fn from(t: AllFileTree) -> Self {
        Self {
            id: t.file.id,
            name: t.file.name.clone(),
            path: t.file.path.clone(),
            is_directory: t.file.is_directory,
            size: t.file.size_bytes,
            modified: t.file.modified_at.to_rfc3339(),
            last_accessed: t.file.last_accessed.map(|dt| dt.to_rfc3339()),
            access_count: t.file.access_count,
            children: t.children.map(|c| c.into_iter().map(AllFileEntryResponse::from).collect()),
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

#[derive(Debug, Deserialize)]
pub struct UploadWithPathRequest {
    #[serde(default)]
    pub parent_path: Option<String>,
}

/// GET /all-files - Get all files as a nested tree
pub async fn get_all_files(
    State(state): State<UnifiedAppState>,
) -> Result<impl IntoResponse, StatusCode> {
    let all_files = &state.shared_state.database_pool.all_files;

    match all_files.get_file_tree() {
        Ok(tree) => {
            let response: Vec<AllFileEntryResponse> = tree.into_iter()
                .map(AllFileEntryResponse::from)
                .collect();
            Ok(Json(response))
        }
        Err(e) => {
            error!("Failed to get all files: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// GET /all-files/:id - Get single file metadata
pub async fn get_all_file_by_id(
    State(state): State<UnifiedAppState>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, StatusCode> {
    let all_files = &state.shared_state.database_pool.all_files;

    match all_files.get_file(id) {
        Ok(file) => Ok(Json(AllFileEntryResponse::from(file))),
        Err(e) => {
            error!("Failed to get all file {}: {}", id, e);
            Err(StatusCode::NOT_FOUND)
        }
    }
}

/// GET /all-files/:id/content - Get file content for LLM processing
pub async fn get_all_file_content(
    State(state): State<UnifiedAppState>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, StatusCode> {
    let all_files = &state.shared_state.database_pool.all_files;

    match all_files.get_file_content_string(id) {
        Ok(content) => {
            let _ = all_files.record_access(id);
            Ok(Json(serde_json::json!({
                "id": id,
                "content": content
            })))
        }
        Err(e) => {
            error!("Failed to get all file content {}: {}", id, e);
            Err(StatusCode::NOT_FOUND)
        }
    }
}

/// GET /all-files/search?q=... - Search files by name
pub async fn search_all_files(
    State(state): State<UnifiedAppState>,
    Query(query): Query<SearchQuery>,
) -> Result<impl IntoResponse, StatusCode> {
    let all_files = &state.shared_state.database_pool.all_files;

    match all_files.search_files(&query.q) {
        Ok(files) => {
            let response: Vec<AllFileEntryResponse> = files.into_iter()
                .map(AllFileEntryResponse::from)
                .collect();
            Ok(Json(response))
        }
        Err(e) => {
            error!("Failed to search all files: {}", e);
            Ok(Json(Vec::<AllFileEntryResponse>::new()))
        }
    }
}

/// GET /all-files/all - Get flat list of all files (for @filename autocomplete)
pub async fn get_all_files_flat(
    State(state): State<UnifiedAppState>,
) -> Result<impl IntoResponse, StatusCode> {
    let all_files = &state.shared_state.database_pool.all_files;

    match all_files.get_all_files() {
        Ok(files) => {
            let response: Vec<AllFileEntryResponse> = files.into_iter()
                .map(AllFileEntryResponse::from)
                .collect();
            Ok(Json(response))
        }
        Err(e) => {
            error!("Failed to get all files: {}", e);
            Ok(Json(Vec::<AllFileEntryResponse>::new()))
        }
    }
}

/// POST /all-files/folder - Create a new folder
pub async fn create_all_files_folder(
    State(state): State<UnifiedAppState>,
    Json(request): Json<CreateFolderRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    let all_files = &state.shared_state.database_pool.all_files;

    if request.name.trim().is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    match all_files.create_folder(request.parent_id, &request.name) {
        Ok(folder) => {
            info!("Created all_files folder: {}", folder.path);
            Ok(Json(serde_json::json!({
                "message": "Folder created successfully",
                "folder": AllFileEntryResponse::from(folder)
            })))
        }
        Err(e) => {
            error!("Failed to create all_files folder: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// POST /all-files/upload - Upload files (supports folder structure via webkitdirectory)
pub async fn upload_all_file(
    State(state): State<UnifiedAppState>,
    Query(query): Query<UploadQuery>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, StatusCode> {
    let all_files = &state.shared_state.database_pool.all_files;
    let parent_id = query.parent_id;
    
    let mut files_to_upload: Vec<(Option<String>, String, Vec<u8>)> = Vec::new();

    // Collect multipart fields
    let mut field_count = 0;
    let mut has_error = false;
    loop {
        match multipart.next_field().await {
            Ok(Some(field)) => {
                field_count += 1;
                let filename = field.file_name()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| format!("file{}", files_to_upload.len()));

                // Get relative path from webkitdirectory uploads BEFORE reading bytes
                let relative_path = field
                    .headers()
                    .get("webkitrelativepath")
                    .and_then(|h| h.to_str().ok())
                    .map(|s| {
                        // Remove the filename from the path to get just the directory
                        let path = std::path::Path::new(s);
                        let parent = path.parent()
                            .map(|p| p.to_string_lossy().to_string())
                            .unwrap_or_default();
                        // Normalize path separators to forward slashes for consistency
                        parent.replace('\\', "/")
                    });

                match field.bytes().await {
                    Ok(data) => {
                        let size = data.len();
                        info!("Processing file: {} ({} bytes), relative_path: {:?}", filename, size, relative_path);
                        files_to_upload.push((relative_path, filename, data.to_vec()));
                    }
                    Err(e) => {
                        error!("Failed to read file data for {}: {}", filename, e);
                        has_error = true;
                    }
                }
            }
            Ok(None) => break, // No more fields
            Err(e) => {
                error!("Error reading multipart field: {}", e);
                has_error = true;
                break;
            }
        }
    }

    info!("Total fields received: {}, files to upload: {}", field_count, files_to_upload.len());

    if files_to_upload.is_empty() {
        if has_error {
            error!("Upload failed due to errors reading fields");
            return Err(StatusCode::BAD_REQUEST);
        }
        warn!("No files to upload - received {} fields but 0 parseable files", field_count);
        return Err(StatusCode::BAD_REQUEST);
    }

    // Use the structure upload function to handle folders
    info!("Uploading {} files with structure, parent_id: {:?}", files_to_upload.len(), parent_id);
    match all_files.upload_files_with_structure(files_to_upload, parent_id) {
        Ok(uploaded) => {
            let response: Vec<AllFileEntryResponse> = uploaded.into_iter()
                .map(AllFileEntryResponse::from)
                .collect();
            info!("Uploaded {} files with structure", response.len());
            Ok(Json(serde_json::json!({
                "message": format!("Uploaded {} file(s) successfully", response.len()),
                "files": response
            })))
        }
        Err(e) => {
            error!("Failed to upload files: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// POST /all-files/upload-structure - Upload files with folder structure
pub async fn upload_all_files_structure(
    State(state): State<UnifiedAppState>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, StatusCode> {
    let all_files = &state.shared_state.database_pool.all_files;
    let mut files_to_upload: Vec<(Option<String>, String, Vec<u8>)> = Vec::new();

    while let Some(field_result) = multipart.next_field().await.ok().flatten() {
        let filename = field_result.file_name()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "file".to_string());

        let relative_path = field_result
            .headers()
            .get("relative-path")
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string());

        let data = field_result.bytes().await.map_err(|_| StatusCode::BAD_REQUEST)?;

        files_to_upload.push((relative_path, filename, data.to_vec()));
    }

    if files_to_upload.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    match all_files.upload_files_with_structure(files_to_upload, None) {
        Ok(uploaded) => {
            let response: Vec<AllFileEntryResponse> = uploaded.into_iter()
                .map(AllFileEntryResponse::from)
                .collect();
            info!("Uploaded {} files with structure", response.len());
            Ok(Json(serde_json::json!({
                "message": format!("Uploaded {} file(s) successfully", response.len()),
                "files": response
            })))
        }
        Err(e) => {
            error!("Failed to upload files with structure: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// DELETE /all-files/:id - Delete file or folder by ID
pub async fn delete_all_file_by_id(
    State(state): State<UnifiedAppState>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, StatusCode> {
    let all_files = &state.shared_state.database_pool.all_files;

    match all_files.delete_file(id) {
        Ok(_) => {
            info!("Deleted all file id: {}", id);
            Ok(Json(serde_json::json!({
                "message": "File deleted successfully"
            })))
        }
        Err(e) => {
            error!("Failed to delete all file {}: {}", id, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// DELETE /all-files?path=... or ?id=... - Delete file by path or ID
pub async fn delete_all_file(
    State(state): State<UnifiedAppState>,
    Query(query): Query<DeleteQuery>,
) -> Result<impl IntoResponse, StatusCode> {
    let all_files = &state.shared_state.database_pool.all_files;

    if let Some(id) = query.id {
        match all_files.delete_file(id) {
            Ok(_) => {
                info!("Deleted all file id: {}", id);
                Ok(Json(serde_json::json!({
                    "message": "File deleted successfully"
                })))
            }
            Err(e) => {
                error!("Failed to delete all file by id {}: {}", id, e);
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    } else if let Some(path) = query.path {
        match all_files.get_file_by_path(&path) {
            Ok(file) => {
                match all_files.delete_file(file.id) {
                    Ok(_) => {
                        info!("Deleted all file: {}", path);
                        Ok(Json(serde_json::json!({
                            "message": "File deleted successfully"
                        })))
                    }
                    Err(e) => {
                        error!("Failed to delete all file {}: {}", path, e);
                        Err(StatusCode::INTERNAL_SERVER_ERROR)
                    }
                }
            }
            Err(e) => {
                error!("All file not found: {}: {}", path, e);
                Err(StatusCode::NOT_FOUND)
            }
        }
    } else {
        Err(StatusCode::BAD_REQUEST)
    }
}
