use std::path::Path;
use std::fs;
use std::io::{Read, Cursor};
use tracing::{debug, info};
use anyhow::Result;

// macOS: Core Graphics PDF C API — always available on macOS regardless of chip
// (CoreGraphics framework is already linked transitively by the core-graphics crate)
#[cfg(target_os = "macos")]
use core_graphics::geometry::CGRect;

#[cfg(target_os = "macos")]
extern "C" {
    fn CGPDFDocumentCreateWithProvider(provider: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
    fn CGPDFDocumentGetNumberOfPages(document: *mut std::ffi::c_void) -> usize;
    fn CGPDFDocumentGetPage(document: *mut std::ffi::c_void, page_index: usize) -> *mut std::ffi::c_void;
    fn CGPDFPageGetBoxRect(page: *mut std::ffi::c_void, box_type: i32) -> CGRect;
    fn CGContextDrawPDFPage(context: *mut std::ffi::c_void, page: *mut std::ffi::c_void);
    fn CGContextScaleCTM(context: *mut std::ffi::c_void, sx: f64, sy: f64);
    fn CGContextTranslateCTM(context: *mut std::ffi::c_void, tx: f64, ty: f64);
    fn CGPDFDocumentRelease(document: *mut std::ffi::c_void);
    fn CGPDFPageRelease(page: *mut std::ffi::c_void);
}

/// Returns `true` when `file_processor` returned a sentinel error string instead of
/// real content. Sentinels always start with `[` and describe a failure.
/// Used by both `stream_api` (cache-hit guard) and `attachment_api` (preprocess guard).
pub fn is_extraction_sentinel(s: &str) -> bool {
    s.starts_with("[Could not")
        || s.starts_with("[PDF")
        || s.starts_with("[DOCX")
        || s.starts_with("[Spreadsheet")
        || s.starts_with("[Presentation")
        || s.starts_with("[ODT")
}

/// Rough token estimate: 1 token ≈ 4 characters (common approximation).
pub fn estimate_tokens(text: &str) -> usize {
    (text.len() + 3) / 4
}

/// Truncate `text` so it fits within `max_tokens`, breaking at the last newline
/// before the limit to avoid cutting mid-sentence.
///
/// Returns `(truncated_text, was_truncated)`.
pub fn truncate_to_budget(text: &str, max_tokens: usize) -> (String, bool) {
    let max_chars = max_tokens.saturating_mul(4);
    if text.len() <= max_chars {
        return (text.to_string(), false);
    }
    // Truncate byte-safe: find last char boundary at or before max_chars
    let mut end = max_chars;
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    let slice = &text[..end];
    // Break at the last newline so we don't cut inside a line
    let cut = slice.rfind('\n').unwrap_or(end);
    (slice[..cut].to_string(), true)
}

/// Extract text content from various file formats
pub async fn extract_file_content(file_path: &Path) -> Result<String> {
    let file_ext = file_path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_lowercase())
        .unwrap_or_default();

    match file_ext.as_str() {
        // Text files
        "txt" | "md" | "json" | "yaml" | "yml" | "xml" | "csv" | "log" => {
            extract_text_file(file_path).await
        },
        // Code files
        "js" | "ts" | "jsx" | "tsx" | "py" | "java" | "cpp" | "c" | "cs" | 
        "html" | "css" | "scss" | "go" | "rs" | "php" | "rb" | "swift" | 
        "kt" | "scala" | "sql" | "sh" | "bat" | "ps1" | "dockerfile" | "env" => {
            extract_text_file(file_path).await
        },
        // Document files
        "pdf" => extract_pdf_content(file_path).await,
        "doc" | "docx" => extract_docx_content(file_path).await,
        "rtf" => extract_text_file(file_path).await,
        "odt" => extract_odt_content(file_path).await,
        // Spreadsheet files
        "xls" | "xlsx" | "ods" => extract_xlsx_content(file_path).await,
        // Presentation files
        "ppt" | "pptx" | "odp" => extract_pptx_content(file_path).await,
        // Default to text extraction
        _ => {
            debug!("Unknown file type {}, attempting text extraction", file_ext);
            extract_text_file(file_path).await
        }
    }
}

/// Extract text from bytes with file extension
pub async fn extract_content_from_bytes(bytes: &[u8], filename: &str) -> Result<String> {
    let ext = filename.split('.').last().unwrap_or("").to_lowercase();

    match ext.as_str() {
        // Text/code files - try UTF-8 decoding
        "txt" | "md" | "json" | "yaml" | "yml" | "xml" | "csv" | "log" |
        "js" | "ts" | "jsx" | "tsx" | "py" | "java" | "cpp" | "c" | "cs" |
        "html" | "css" | "scss" | "go" | "rs" | "php" | "rb" | "swift" |
        "kt" | "scala" | "sql" | "sh" | "bat" | "ps1" | "dockerfile" | "env" | "rtf" => {
            Ok(String::from_utf8_lossy(bytes).to_string())
        },
        // PDF files — run OCR on a blocking thread to avoid starving the async runtime
        "pdf" => {
            let bytes_owned = bytes.to_vec();
            let text = tokio::task::spawn_blocking(move || extract_pdf_from_bytes(&bytes_owned))
                .await
                .unwrap_or_else(|_| "[PDF extraction panicked]".to_string());
            Ok(text)
        },
        // Word documents
        "doc" | "docx" => Ok(extract_docx_from_bytes(bytes)),
        // OpenDocument text
        "odt" => Ok(extract_odt_from_bytes(bytes)),
        // Spreadsheets
        "xls" | "xlsx" | "ods" => Ok(extract_xlsx_from_bytes(bytes, &ext)),
        // Presentations
        "ppt" | "pptx" | "odp" => Ok(extract_pptx_from_bytes(bytes)),
        // Default - try text
        _ => {
            debug!("Unknown file type {}, attempting text extraction", ext);
            Ok(String::from_utf8_lossy(bytes).to_string())
        }
    }
}

/// Extract content from text-based files
async fn extract_text_file(file_path: &Path) -> Result<String> {
    let content = fs::read_to_string(file_path)?;
    Ok(content)
}

/// Extract content from PDF files
async fn extract_pdf_content(file_path: &Path) -> Result<String> {
    let bytes = fs::read(file_path)?;
    // Run OCR (blocking WinRT/Vision calls) on a dedicated blocking thread
    let text = tokio::task::spawn_blocking(move || extract_pdf_from_bytes(&bytes))
        .await
        .unwrap_or_else(|_| "[PDF extraction panicked]".to_string());
    Ok(text)
}

/// Try to extract the embedded text layer from a PDF using pure Rust (no OCR required).
///
/// Works for text-based PDFs produced by Word, Google Docs, LibreOffice, LaTeX, etc.
/// Returns `None` for scanned / image-only PDFs (no text layer) or on parse failure.
///
/// This is cross-platform and avoids the OS-specific OCR engines entirely for the
/// majority of PDFs that users actually attach (digital documents, not scans).
fn extract_pdf_text_layer(bytes: &[u8]) -> Option<String> {
    let doc = lopdf::Document::load_mem(bytes).ok()?;
    let page_count = doc.get_pages().len();
    if page_count == 0 {
        return None;
    }

    // Primary: extract all pages at once (fast path)
    let page_numbers: Vec<u32> = (1..=page_count as u32).collect();
    let full_text = doc.extract_text(&page_numbers).ok();

    // Fallback: if full extraction fails or returns empty, try page-by-page
    // (handles some PDFs where certain pages fail to decode as a batch)
    let text = match full_text {
        Some(ref t) if !t.trim().is_empty() => t.clone(),
        _ => {
            debug!("Full-document lopdf extraction returned empty — trying page-by-page");
            let mut page_text = String::new();
            for page_num in 1..=page_count as u32 {
                if let Ok(t) = doc.extract_text(&[page_num]) {
                    page_text.push_str(t.trim());
                    page_text.push('\n');
                }
            }
            page_text
        }
    };

    let trimmed = text.trim().to_string();
    if trimmed.is_empty() {
        return None;
    }

    // Sanity-check: if fewer than 40% of characters are printable the text layer
    // is likely garbage from a non-standard font encoding — fall back to OCR.
    // Threshold relaxed from 60% → 40% to handle technical PDFs with many
    // non-ASCII symbols (math formulae, source code with special chars, etc.).
    let total = trimmed.chars().count();
    if total > 0 {
        let printable = trimmed
            .chars()
            .filter(|c| !c.is_control() || matches!(*c, '\n' | '\r' | '\t'))
            .count();
        if printable * 100 / total < 40 {
            info!(
                "PDF text layer looks garbled ({}/{} printable chars) — will try OCR fallback",
                printable, total
            );
            return None;
        }
    }

    info!("PDF text layer extracted ({} chars, {} pages) — no OCR needed", trimmed.len(), page_count);
    Some(trimmed)
}

fn extract_pdf_from_bytes(bytes: &[u8]) -> String {
    // ── 1. Fast path: pure Rust text-layer extraction ────────────────────────
    // Works for text-based PDFs (Word/Google Docs exports, LaTeX, etc.).
    // Cross-platform — no OS OCR engine required.
    if let Some(text) = extract_pdf_text_layer(bytes) {
        return text;
    }

    info!("PDF has no extractable text layer — attempting OS-native OCR");

    // ── 2. Slow path: OS-native OCR (for scanned / image-based PDFs) ─────────
    #[cfg(target_os = "windows")]
    {
        match windows_ocr_pdf(bytes) {
            Some(text) if !text.trim().is_empty() => {
                info!("PDF extracted via Windows OCR ({} chars)", text.len());
                return text;
            }
            Some(_) => {
                info!("Windows OCR returned empty text — PDF may be purely image-based");
            }
            None => {
                info!("Windows OCR unavailable or failed — PDF may be encrypted or corrupted");
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        match macos_ocr_pdf(bytes) {
            Some(text) if !text.trim().is_empty() => {
                info!("PDF extracted via macOS Vision OCR ({} chars)", text.len());
                return text;
            }
            Some(_) => {
                info!("macOS OCR returned empty text — PDF may be purely image-based");
            }
            None => {
                info!("macOS OCR unavailable or failed — PDF may be encrypted or corrupted");
            }
        }
    }

    // All strategies exhausted
    "[PDF extraction failed. This file appears to be scanned, encrypted, or corrupted. \
Please try: 1) Save as text-based PDF, 2) Use DOCX format, or 3) Paste text directly]".to_string()
}

// ── Windows OCR ──────────────────────────────────────────────────────────────

/// Initialise WinRT on the calling thread (once per thread, idempotent).
/// Called before any WinRT API use to ensure the thread has a COM apartment.
#[cfg(target_os = "windows")]
fn ensure_winrt_init() {
    thread_local! {
        static INIT: () = {
            unsafe {
                // S_OK = 0 (fresh init), S_FALSE = 1 (already init on this thread),
                // RPC_E_CHANGED_MODE = STA thread — all are safe to ignore.
                let _ = windows::Win32::System::WinRT::RoInitialize(
                    windows::Win32::System::WinRT::RO_INIT_MULTITHREADED,
                );
            }
        };
    }
    INIT.with(|_| ());
}

/// Render each PDF page with Windows.Data.Pdf and run Windows.Media.Ocr on it.
/// Returns `None` when the engine is unavailable or no text is found.
#[cfg(target_os = "windows")]
fn windows_ocr_pdf(bytes: &[u8]) -> Option<String> {
    use windows::{
        core::*,
        Data::Pdf::PdfDocument,
        Graphics::Imaging::{BitmapDecoder, BitmapPixelFormat, SoftwareBitmap},
        Media::Ocr::OcrEngine,
        Storage::Streams::{DataWriter, IOutputStream, IRandomAccessStream, InMemoryRandomAccessStream},
    };

    info!("Starting Windows OCR for PDF ({} bytes)", bytes.len());

    ensure_winrt_init();
    info!("WinRT initialized successfully");

    let run = || -> windows::core::Result<String> {
        // ── 1. Write PDF bytes into an in-memory random-access stream ─────────
        info!("Creating in-memory PDF stream");
        let pdf_stream = InMemoryRandomAccessStream::new()?;
        {
            let writer = DataWriter::new()?;
            writer.WriteBytes(bytes)?;
            let buffer = writer.DetachBuffer()?;
            let out: IOutputStream = pdf_stream.cast()?;
            out.WriteAsync(&buffer)?.get()?;
            out.FlushAsync()?.get()?;
        }
        pdf_stream.Seek(0)?;
        info!("PDF stream created successfully");

        // ── 2. Load the PDF document ──────────────────────────────────────────
        info!("Loading PDF document");
        let pdf_doc = PdfDocument::LoadFromStreamAsync(&pdf_stream)?.get()?;
        let page_count = pdf_doc.PageCount()?;
        info!("PDF loaded, {} pages", page_count);
        
        if page_count == 0 {
            return Ok(String::new());
        }

        // ── 3. Create OCR engine (uses the user's Windows language profile) ───
        info!("Creating OCR engine");
        let ocr_engine = OcrEngine::TryCreateFromUserProfileLanguages()?;
        info!("OCR engine created successfully");

        // ── 4. Render each page → PNG → SoftwareBitmap → OCR ─────────────────
        let mut all_text = String::new();

        for page_idx in 0..page_count {
            info!("Processing page {}/{}", page_idx + 1, page_count);
            let page = pdf_doc.GetPage(page_idx)?;

            // Render page to PNG in memory
            let img_stream = InMemoryRandomAccessStream::new()?;
            let img_iras: IRandomAccessStream = img_stream.cast()?;
            page.RenderToStreamAsync(&img_iras)?.get()?;
            img_stream.Seek(0)?;
            info!("Page {} rendered to stream", page_idx);

            // Decode PNG to SoftwareBitmap (auto-detects format — no codec ID needed in 0.52)
            let decoder = BitmapDecoder::CreateAsync(&img_iras)?.get()?;
            let bitmap = decoder.GetSoftwareBitmapAsync()?.get()?;

            // OcrEngine requires Bgra8 pixel format
            let bitmap = if bitmap.BitmapPixelFormat()? != BitmapPixelFormat::Bgra8 {
                SoftwareBitmap::Convert(&bitmap, BitmapPixelFormat::Bgra8)?
            } else {
                bitmap
            };

            // Recognise text on this page
            match ocr_engine.RecognizeAsync(&bitmap)?.get() {
                Ok(result) => {
                    let text = result.Text()?.to_string();
                    if !text.trim().is_empty() {
                        all_text.push_str(&text);
                        all_text.push('\n');
                        info!("Extracted {} chars from page {}", text.len(), page_idx);
                    }
                }
                Err(e) => info!("OCR page {} error: {}", page_idx, e),
            }
        }

        info!("Windows OCR complete, total chars: {}", all_text.len());
        Ok(all_text)
    };

    match run() {
        Ok(text) if !text.trim().is_empty() => Some(text),
        Ok(_) => {
            info!("Windows OCR: no text found in PDF");
            None
        }
        Err(e) => {
            info!("Windows OCR failed: {}", e);
            None
        }
    }
}

// ── macOS OCR ──────────────────────────────────────────────────────────────

/// Render each PDF page with Core Graphics and run Vision OCR on the result.
/// Works on both Apple Silicon (ARM64, uses Neural Engine) and Intel (x86_64, uses CPU).
/// Returns `None` when the PDF is empty, unreadable, or yields no text.
#[cfg(target_os = "macos")]
fn macos_ocr_pdf(bytes: &[u8]) -> Option<String> {
    use std::ffi::c_void;
    use std::sync::Arc;
    use core_graphics::{
        color_space::CGColorSpace,
        context::CGContext,
        data_provider::CGDataProvider,
    };
    use objc2::rc::Retained;
    use objc2_foundation::{NSArray, NSData, NSDictionary, NSString};
    use objc2_vision::{
        VNImageRequestHandler, VNRecognizeTextRequest,
        VNRequest, VNRequestTextRecognitionLevel,
    };

    // kCGImageAlphaNoneSkipLast (4) — RGBX pixel format, ignore the 4th byte as alpha.
    // Using a plain u32 avoids depending on a specific CGBitmapInfo constant path.
    const BITMAP_INFO: u32 = 4;

    let run = || -> Result<String, String> {
        // ── 1. Load PDF bytes into a CGPDFDocument ───────────────────────────
        //
        // CGDataProvider::from_buffer keeps the Arc alive for the provider's lifetime,
        // so the underlying bytes are valid for as long as we need the document.
        let pdf_data: Arc<Vec<u8>> = Arc::new(bytes.to_vec());
        let provider = CGDataProvider::from_buffer(pdf_data);

        let doc = unsafe {
            CGPDFDocumentCreateWithProvider(provider.as_ptr() as *mut c_void)
        };
        if doc.is_null() {
            return Err("CGPDFDocumentCreateWithProvider returned null".into());
        }

        let page_count = unsafe { CGPDFDocumentGetNumberOfPages(doc) };
        if page_count == 0 {
            unsafe { CGPDFDocumentRelease(doc) };
            return Ok(String::new());
        }

        info!("macOS PDF OCR: {} page(s)", page_count);
        let mut all_text = String::new();

        // ── 2. Per-page: render → PNG → Vision OCR ──────────────────────────
        //
        // CGPDFDocument uses 1-based page indexing.
        for page_idx in 1..=page_count {
            let page = unsafe { CGPDFDocumentGetPage(doc, page_idx) };
            if page.is_null() {
                continue;
            }

            // PDF page dimensions are in points (72 pt = 1 inch).
            // kCGPDFMediaBox = 0 — the full physical page rectangle.
            let media_box = unsafe { CGPDFPageGetBoxRect(page, 0) };
            let pt_w = media_box.size.width;
            let pt_h = media_box.size.height;

            // Scale to 150 DPI: good balance between OCR accuracy and memory.
            let scale = 150.0_f64 / 72.0;
            let px_w = ((pt_w * scale).ceil() as usize).max(1);
            let px_h = ((pt_h * scale).ceil() as usize).max(1);
            let bytes_per_row = px_w * 4; // 4 bytes/pixel (RGBX)

            // Allocate a white pixel buffer — pages with transparency get a white bg.
            let mut pixel_buf = vec![255u8; bytes_per_row * px_h];

            let color_space = CGColorSpace::create_device_rgb();
            let ctx = unsafe {
                CGContext::create_bitmap_context(
                    Some(pixel_buf.as_mut_ptr() as *mut c_void),
                    px_w,
                    px_h,
                    8,             // bits per component
                    bytes_per_row,
                    &color_space,
                    BITMAP_INFO,
                )
            };
            let ctx_ptr = ctx.as_ptr() as *mut c_void;

            // PDF coordinate origin is bottom-left; CGBitmapContext origin is top-left.
            // Flip Y: translate to the top of the context, then negate the Y scale.
            unsafe {
                CGContextTranslateCTM(ctx_ptr, 0.0, px_h as f64);
                CGContextScaleCTM(ctx_ptr, scale, -scale);
                CGContextDrawPDFPage(ctx_ptr, page);
            }
            unsafe { CGPDFPageRelease(page) };

            // Drop the context to flush any deferred drawing before reading pixel_buf.
            drop(ctx);

            // ── 3. Encode rendered pixels to PNG ──────────────────────────────
            //
            // Vision's initWithData:options: accepts any image format that NSImage
            // can decode (PNG, JPEG, TIFF, …).  PNG is lossless and zero-dependency.
            let mut png_bytes: Vec<u8> = Vec::new();
            {
                let mut enc = png::Encoder::new(&mut png_bytes, px_w as u32, px_h as u32);
                enc.set_color(png::ColorType::Rgba);
                enc.set_depth(png::BitDepth::Eight);
                enc.write_header()
                    .and_then(|mut w| w.write_image_data(&pixel_buf))
                    .map_err(|e| format!("PNG encode failed on page {page_idx}: {e}"))?;
            }

            info!(
                "Page {page_idx} rendered {px_w}×{px_h} px ({} PNG bytes)",
                png_bytes.len()
            );

            // ── 4. Vision OCR ──────────────────────────────────────────────────
            unsafe {
                let ns_data = NSData::with_bytes(&png_bytes);
                let options = NSDictionary::<NSString, objc2::runtime::AnyObject>::new();

                let handler = VNImageRequestHandler::initWithData_options(
                    VNImageRequestHandler::alloc(),
                    &ns_data,
                    &options,
                );

                let request =
                    VNRecognizeTextRequest::init(VNRecognizeTextRequest::alloc());

                // Accurate mode uses the Neural Engine on Apple Silicon;
                // falls back to CPU on Intel — both handled transparently by Vision.
                request.setRecognitionLevel(VNRequestTextRecognitionLevel::Accurate);
                request.setUsesLanguageCorrection(true);

                // performRequests:error: expects NSArray<VNRequest>.
                // VNRecognizeTextRequest IS-A VNRequest via Objective-C inheritance.
                // Rust deref coercion traverses: Retained<VNRecognizeTextRequest>
                //   → VNRecognizeTextRequest → VNImageBasedRequest → VNRequest
                let req_as_base: &VNRequest = &*request;
                let req_array = NSArray::from_slice(&[req_as_base]);

                // Ignore the return value; if it fails, results() will be None.
                let _ = handler.performRequests_error(&*req_array);

                if let Some(results) = request.results() {
                    for obs in results.iter() {
                        // topCandidates(1) returns the single best candidate string.
                        let candidates = obs.topCandidates(1);
                        if let Some(top) = candidates.firstObject() {
                            let text = top.string().to_string();
                            if !text.is_empty() {
                                all_text.push_str(&text);
                                all_text.push('\n');
                            }
                        }
                    }
                }
            }
        }

        unsafe { CGPDFDocumentRelease(doc) };
        info!("macOS PDF OCR complete: {} chars", all_text.len());
        Ok(all_text)
    };

    match run() {
        Ok(text) if !text.trim().is_empty() => Some(text),
        Ok(_) => {
            debug!("macOS OCR: no text found in PDF");
            None
        }
        Err(e) => {
            debug!("macOS OCR failed: {e}");
            None
        }
    }
}

/// Extract content from DOCX files
async fn extract_docx_content(file_path: &Path) -> Result<String> {
    let bytes = fs::read(file_path)?;
    Ok(extract_docx_from_bytes(&bytes))
}

fn extract_docx_from_bytes(bytes: &[u8]) -> String {
    let cursor = Cursor::new(bytes);
    match zip::ZipArchive::new(cursor) {
        Ok(mut archive) => {
            if let Ok(mut file) = archive.by_name("word/document.xml") {
                let mut xml = String::new();
                if file.read_to_string(&mut xml).is_ok() {
                    let text = xml_to_plain_text(&xml, "</w:p>", "</w:tr>");
                    if text.is_empty() {
                        "[DOCX file appears to be empty]".to_string()
                    } else {
                        text
                    }
                } else {
                    "[Could not read DOCX content]".to_string()
                }
            } else {
                "[Could not find document content in DOCX file]".to_string()
            }
        }
        Err(e) => {
            debug!("DOCX extraction failed: {}", e);
            format!("[Could not extract DOCX content: {}]", e)
        }
    }
}

/// General XML → plain text helper used for DOCX, PPTX, and ODT.
///
/// `paragraph_end` and `row_end` are the XML closing tags that should become
/// newlines before all other tags are stripped.
fn xml_to_plain_text(xml: &str, paragraph_end: &str, row_end: &str) -> String {
    // Insert newlines at structural boundaries before stripping all tags
    let s = xml
        .replace(paragraph_end, "\n")
        .replace(row_end, "\n");

    // Strip all remaining XML tags
    let tag_re = regex::Regex::new(r"<[^>]+>").unwrap();
    let plain = tag_re.replace_all(&s, "");

    // Decode the most common XML/HTML entities
    let plain = plain
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&#x9;", "\t")
        .replace("&#xA;", "\n")
        .replace("&#xD;", "");

    // Normalise: trim each line, drop lines that are only whitespace,
    // collapse more than one consecutive blank line into a single blank line
    let mut result = String::new();
    let mut blank_run = 0usize;
    for line in plain.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            blank_run += 1;
            if blank_run == 1 {
                result.push('\n');
            }
        } else {
            blank_run = 0;
            result.push_str(trimmed);
            result.push('\n');
        }
    }

    result.trim().to_string()
}

/// Extract content from XLSX files
async fn extract_xlsx_content(file_path: &Path) -> Result<String> {
    use calamine::{Reader, open_workbook_auto};
    
    match open_workbook_auto(file_path) {
        Ok(mut workbook) => {
            let mut text = String::new();
            for sheet_name in workbook.sheet_names().to_vec() {
                if let Ok(range) = workbook.worksheet_range(&sheet_name) {
                    text.push_str(&format!("\n=== Sheet: {} ===\n", sheet_name));
                    for row in range.rows() {
                        let row_text: Vec<String> = row.iter().map(|c| c.to_string()).collect();
                        text.push_str(&row_text.join("\t"));
                        text.push('\n');
                    }
                }
            }
            if text.trim().is_empty() {
                Ok("[Spreadsheet appears to be empty]".to_string())
            } else {
                Ok(text)
            }
        }
        Err(e) => {
            debug!("XLSX extraction failed: {}", e);
            Ok(format!("[Could not extract spreadsheet content: {}]", e))
        }
    }
}

fn extract_xlsx_from_bytes(bytes: &[u8], ext: &str) -> String {
    use calamine::{Reader, Xls, Xlsx, Ods};

    let mut text = String::new();

    match ext {
        "ods" => {
            let cursor = Cursor::new(bytes);
            if let Ok(mut workbook) = Ods::new(cursor) {
                for sheet_name in workbook.sheet_names().to_vec() {
                    if let Ok(range) = workbook.worksheet_range(&sheet_name) {
                        text.push_str(&format!("\n=== Sheet: {} ===\n", sheet_name));
                        for row in range.rows() {
                            let row_text: Vec<String> = row.iter().map(|c| c.to_string()).collect();
                            text.push_str(&row_text.join("\t"));
                            text.push('\n');
                        }
                    }
                }
            }
        }
        "xls" => {
            // OLE2/BIFF format — requires calamine::Xls, not calamine::Xlsx
            let cursor = Cursor::new(bytes);
            if let Ok(mut workbook) = Xls::new(cursor) {
                for sheet_name in workbook.sheet_names().to_vec() {
                    if let Ok(range) = workbook.worksheet_range(&sheet_name) {
                        text.push_str(&format!("\n=== Sheet: {} ===\n", sheet_name));
                        for row in range.rows() {
                            let row_text: Vec<String> = row.iter().map(|c| c.to_string()).collect();
                            text.push_str(&row_text.join("\t"));
                            text.push('\n');
                        }
                    }
                }
            }
        }
        _ => {
            // xlsx, xlsb — OOXML ZIP format
            let cursor = Cursor::new(bytes);
            if let Ok(mut workbook) = Xlsx::new(cursor) {
                for sheet_name in workbook.sheet_names().to_vec() {
                    if let Ok(range) = workbook.worksheet_range(&sheet_name) {
                        text.push_str(&format!("\n=== Sheet: {} ===\n", sheet_name));
                        for row in range.rows() {
                            let row_text: Vec<String> = row.iter().map(|c| c.to_string()).collect();
                            text.push_str(&row_text.join("\t"));
                            text.push('\n');
                        }
                    }
                }
            }
        }
    }

    if text.trim().is_empty() {
        "[Spreadsheet appears to be empty or could not be read]".to_string()
    } else {
        text
    }
}

/// Extract content from PPTX files
async fn extract_pptx_content(file_path: &Path) -> Result<String> {
    let bytes = fs::read(file_path)?;
    Ok(extract_pptx_from_bytes(&bytes))
}

fn extract_pptx_from_bytes(bytes: &[u8]) -> String {
    let cursor = Cursor::new(bytes);
    match zip::ZipArchive::new(cursor) {
        Ok(mut archive) => {
            let mut text = String::new();
            let mut slide_num = 1;

            loop {
                let slide_path = format!("ppt/slides/slide{}.xml", slide_num);
                match archive.by_name(&slide_path) {
                    Ok(mut file) => {
                        let mut xml = String::new();
                        if file.read_to_string(&mut xml).is_ok() {
                            let content = xml_to_plain_text(&xml, "</a:p>", "</a:r>");
                            if !content.is_empty() {
                                text.push_str(&format!("\n=== Slide {} ===\n{}", slide_num, content));
                            }
                        }
                        slide_num += 1;
                    }
                    Err(_) => break,
                }
            }

            if text.trim().is_empty() {
                "[Presentation appears to be empty]".to_string()
            } else {
                text
            }
        }
        Err(e) => {
            debug!("PPTX extraction failed: {}", e);
            format!("[Could not extract presentation content: {}]", e)
        }
    }
}

/// Extract content from ODT files
async fn extract_odt_content(file_path: &Path) -> Result<String> {
    let bytes = fs::read(file_path)?;
    Ok(extract_odt_from_bytes(&bytes))
}

fn extract_odt_from_bytes(bytes: &[u8]) -> String {
    let cursor = Cursor::new(bytes);
    match zip::ZipArchive::new(cursor) {
        Ok(mut archive) => {
            if let Ok(mut file) = archive.by_name("content.xml") {
                let mut xml = String::new();
                if file.read_to_string(&mut xml).is_ok() {
                    let text = xml_to_plain_text(&xml, "</text:p>", "</table:table-row>");
                    if text.is_empty() {
                        "[ODT file appears to be empty]".to_string()
                    } else {
                        text
                    }
                } else {
                    "[Could not read ODT content]".to_string()
                }
            } else {
                "[Could not find content in ODT file]".to_string()
            }
        }
        Err(e) => {
            debug!("ODT extraction failed: {}", e);
            format!("[Could not extract ODT content: {}]", e)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_extract_text_file() {
        let temp_file = NamedTempFile::new().unwrap();
        let content = "Test file content\nwith multiple lines";
        fs::write(&temp_file.path(), content).unwrap();

        let result = extract_text_file(temp_file.path()).await.unwrap();
        assert_eq!(result, content);
    }

    #[tokio::test]
    async fn test_extract_unknown_file_type() {
        let temp_file = NamedTempFile::new().unwrap();
        let content = "Unknown file content";
        fs::write(&temp_file.path(), content).unwrap();

        let result = extract_file_content(temp_file.path()).await.unwrap();
        assert_eq!(result, content);
    }
}
