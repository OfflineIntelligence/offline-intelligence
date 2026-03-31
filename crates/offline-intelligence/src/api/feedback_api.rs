
use axum::{
    extract::Json,
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use tracing::{info, error, warn};

#[derive(Debug, Deserialize)]
pub struct FeedbackRequest {
    pub message: String,
    #[serde(default)]
    pub email: String,
}

#[derive(Debug, Serialize)]
pub struct FeedbackResponse {
    pub success: bool,
    pub message: String,
}

pub async fn submit_feedback(
    Json(payload): Json<FeedbackRequest>,
) -> (StatusCode, Json<FeedbackResponse>) {
    if payload.message.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(FeedbackResponse {
                success: false,
                message: "Feedback message cannot be empty".to_string(),
            }),
        );
    }

    info!("Received feedback submission ({} chars)", payload.message.len());

    if let Err(e) = save_feedback_locally(&payload) {
        error!("Failed to save feedback locally: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(FeedbackResponse {
                success: false,
                message: "Failed to save feedback. Please try again.".to_string(),
            }),
        );
    }

    info!("Feedback saved locally");

    match send_feedback_email(&payload).await {
        Ok(feedback_number) => info!("Feedback email #{} sent to product team", feedback_number),
        Err(e) => warn!("Email not sent (SMTP may not be configured): {}", e),
    }

    (
        StatusCode::OK,
        Json(FeedbackResponse {
            success: true,
            message: "Feedback submitted successfully. Thank you!".to_string(),
        }),
    )
}

fn aud_io_data_dir() -> std::path::PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
        .join("Aud.io")
        .join("data")
}

fn save_feedback_locally(payload: &FeedbackRequest) -> Result<(), Box<dyn std::error::Error>> {
    let db_dir = aud_io_data_dir();
    std::fs::create_dir_all(&db_dir)?;

    let conn = rusqlite::Connection::open(db_dir.join("feedback.db"))?;

    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS feedback (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            message TEXT NOT NULL,
            email TEXT DEFAULT '',
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );"
    )?;

    conn.execute(
        "INSERT INTO feedback (message, email) VALUES (?1, ?2)",
        rusqlite::params![payload.message, payload.email],
    )?;

    Ok(())
}

fn get_next_feedback_number() -> u64 {
    let counter_path = aud_io_data_dir().join("feedback_counter.txt");
    
    let current = if counter_path.exists() {
        std::fs::read_to_string(&counter_path)
            .ok()
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(0)
    } else {
        0
    };
    
    let next = current + 1;
    
    if let Some(parent) = counter_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(counter_path, next.to_string());
    
    next
}

async fn send_feedback_email(payload: &FeedbackRequest) -> Result<u64, Box<dyn std::error::Error>> {
    use lettre::{
        Message, SmtpTransport, Transport,
        message::header::ContentType,
        transport::smtp::authentication::Credentials,
    };

    let smtp_user = std::env::var("SMTP_USER")
        .unwrap_or_else(|_| option_env!("SMTP_USER").unwrap_or("").to_string());
    let smtp_pass = std::env::var("SMTP_PASS")
        .unwrap_or_else(|_| option_env!("SMTP_PASS").unwrap_or("").to_string());

    if smtp_user.is_empty() || smtp_pass.is_empty() {
        return Err("SMTP credentials not configured".into());
    }

    let smtp_host = std::env::var("SMTP_HOST")
        .unwrap_or_else(|_| option_env!("SMTP_HOST").unwrap_or("smtp.hostinger.com").to_string());
    let smtp_port: u16 = std::env::var("SMTP_PORT")
        .unwrap_or_else(|_| option_env!("SMTP_PORT").unwrap_or("587").to_string())
        .parse()
        .unwrap_or(587);

    let feedback_number = get_next_feedback_number();

    let from_address = format!("Aud.io Feedback <{}>", smtp_user);
    
    let mut email_builder = Message::builder()
        .from(from_address.parse()?)
        .to("Product Team <product@offlineintelligence.io>".parse()?)
        .subject(format!("FEEDBACK #{} - _Aud.io User Feedback", feedback_number));
    
    if !payload.email.is_empty() {
        email_builder = email_builder.reply_to(payload.email.parse()?);
    }
    
    let email = email_builder
        .header(ContentType::TEXT_PLAIN)
        .body(format!(
            "FEEDBACK #{}\n\nNew feedback from _Aud.io user:\n\n{}\n\n---\nUser email: {}",
            feedback_number,
            payload.message,
            if payload.email.is_empty() { "Not provided" } else { &payload.email }
        ))?;

    let creds = Credentials::new(smtp_user, smtp_pass);

    let mailer = SmtpTransport::starttls_relay(&smtp_host)?
        .port(smtp_port)
        .credentials(creds)
        .build();

    mailer.send(&email)?;

    Ok(feedback_number)
}
