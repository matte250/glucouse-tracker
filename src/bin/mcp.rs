use std::path::Path;

use chrono::NaiveDateTime;
use glucose_tracker::{db::Database, export::export_pdf_headless};
use rmcp::{
    Error as McpError, ServerHandler, ServiceExt,
    model::{
        CallToolResult, Content, Implementation, ProtocolVersion, ServerCapabilities, ServerInfo,
    },
    tool,
};
use schemars::JsonSchema;
use serde::Deserialize;

const DATETIME_FMT: &str = "%Y-%m-%dT%H:%M:%S";

#[derive(Debug, Clone)]
struct GlucoseServer {
    db_path: String,
}

impl GlucoseServer {
    fn open_db(&self) -> Result<Database, McpError> {
        Database::open(&self.db_path).map_err(|e| {
            McpError::internal_error(format!("Failed to open database: {e}"), None)
        })
    }

    fn parse_dt(s: &str) -> Result<NaiveDateTime, McpError> {
        NaiveDateTime::parse_from_str(s, DATETIME_FMT)
            .map_err(|e| McpError::invalid_params(format!("Invalid datetime '{s}': {e}"), None))
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct AddParams {
    #[schemars(description = "ISO 8601 datetime, e.g. 2024-01-15T08:30:00")]
    datetime: String,
    #[schemars(description = "Glucose value in mmol/L")]
    value: f64,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct RemoveParams {
    #[schemars(description = "ISO 8601 datetime, e.g. 2024-01-15T08:30:00")]
    datetime: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct RangeParams {
    #[schemars(description = "Start datetime (ISO 8601, e.g. 2024-01-01T00:00:00)")]
    from_date: String,
    #[schemars(description = "End datetime (ISO 8601, e.g. 2024-01-31T23:59:59)")]
    to_date: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct ExportParams {
    #[schemars(description = "Start datetime (ISO 8601, e.g. 2024-01-01T00:00:00)")]
    from_date: String,
    #[schemars(description = "End datetime (ISO 8601, e.g. 2024-01-31T23:59:59)")]
    to_date: String,
    #[schemars(description = "Output file path for the PDF")]
    output_path: String,
}

#[tool(tool_box)]
impl GlucoseServer {
    #[tool(description = "Add a glucose reading to the database")]
    async fn add_glucose_reading(
        &self,
        #[tool(aggr)] params: AddParams,
    ) -> Result<CallToolResult, McpError> {
        let dt = Self::parse_dt(&params.datetime)?;
        let db = self.open_db()?;

        db.insert_reading(params.value, dt)
            .map_err(|e| McpError::internal_error(format!("DB error: {e}"), None))?;

        Ok(CallToolResult::success(vec![Content::text(format!(
            "Added reading: {:.1} mmol/L at {}",
            params.value, params.datetime
        ))]))
    }

    #[tool(description = "Remove a glucose reading by exact datetime")]
    async fn remove_glucose_reading(
        &self,
        #[tool(aggr)] params: RemoveParams,
    ) -> Result<CallToolResult, McpError> {
        let dt = Self::parse_dt(&params.datetime)?;
        let db = self.open_db()?;

        let matches = db
            .find_readings_by_datetime(dt)
            .map_err(|e| McpError::internal_error(format!("DB error: {e}"), None))?;

        match matches.len() {
            0 => Ok(CallToolResult::success(vec![Content::text(format!(
                "No reading found at {}",
                params.datetime
            ))])),
            1 => {
                let reading = &matches[0];
                db.delete_reading(reading.id)
                    .map_err(|e| McpError::internal_error(format!("DB error: {e}"), None))?;
                Ok(CallToolResult::success(vec![Content::text(format!(
                    "Deleted reading: {:.1} mmol/L at {}",
                    reading.value, params.datetime
                ))]))
            }
            _ => {
                let list: String = matches
                    .iter()
                    .map(|r| {
                        format!(
                            "  id={} value={:.1} at {}",
                            r.id,
                            r.value,
                            r.recorded_at.format(DATETIME_FMT)
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                Ok(CallToolResult::success(vec![Content::text(format!(
                    "Multiple readings found at {}:\n{}\nPlease use a more specific datetime.",
                    params.datetime, list
                ))]))
            }
        }
    }

    #[tool(description = "List glucose readings in a date/time range")]
    async fn list_glucose_readings(
        &self,
        #[tool(aggr)] params: RangeParams,
    ) -> Result<CallToolResult, McpError> {
        let from = Self::parse_dt(&params.from_date)?;
        let to = Self::parse_dt(&params.to_date)?;
        let db = self.open_db()?;

        let mut readings = db
            .get_readings(Some(from), Some(to))
            .map_err(|e| McpError::internal_error(format!("DB error: {e}"), None))?;

        if readings.is_empty() {
            return Ok(CallToolResult::success(vec![Content::text(format!(
                "No readings found between {} and {}",
                params.from_date, params.to_date
            ))]));
        }

        readings.sort_by_key(|r| r.recorded_at);
        let list: String = readings
            .iter()
            .map(|r| {
                format!(
                    "  {} - {:.1} mmol/L",
                    r.recorded_at.format(DATETIME_FMT),
                    r.value
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        Ok(CallToolResult::success(vec![Content::text(format!(
            "Found {} reading(s):\n{}",
            readings.len(),
            list
        ))]))
    }

    #[tool(description = "Export glucose readings as a PDF report with chart and table")]
    async fn export_glucose_pdf(
        &self,
        #[tool(aggr)] params: ExportParams,
    ) -> Result<CallToolResult, McpError> {
        let from = Self::parse_dt(&params.from_date)?;
        let to = Self::parse_dt(&params.to_date)?;
        let db = self.open_db()?;

        let readings = db
            .get_readings(Some(from), Some(to))
            .map_err(|e| McpError::internal_error(format!("DB error: {e}"), None))?;

        if readings.is_empty() {
            return Ok(CallToolResult::success(vec![Content::text(format!(
                "No readings found between {} and {}",
                params.from_date, params.to_date
            ))]));
        }

        let path = Path::new(&params.output_path);
        export_pdf_headless(path, &readings)
            .map_err(|e| McpError::internal_error(format!("PDF export failed: {e}"), None))?;

        Ok(CallToolResult::success(vec![Content::text(format!(
            "PDF exported to {} with {} reading(s)",
            params.output_path,
            readings.len()
        ))]))
    }
}

impl ServerHandler for GlucoseServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation {
                name: "glucose-tracker".into(),
                version: env!("CARGO_PKG_VERSION").into(),
            },
            instructions: Some(
                "Manage glucose readings: add, remove, list, or export to PDF.".into(),
            ),
        }
    }
}

#[tokio::main]
async fn main() {
    let db_path = std::env::var("GLUCOSE_DB_PATH")
        .unwrap_or_else(|_| "glucose_tracker.db".to_string());

    let server = GlucoseServer { db_path };

    let service = server
        .serve(rmcp::transport::stdio())
        .await
        .expect("Failed to start MCP server");

    service.waiting().await.expect("MCP server error");
}
