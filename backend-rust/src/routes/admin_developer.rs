use std::{
    process::Stdio,
    time::{Duration, Instant},
};

use axum::{
    Json,
    extract::{Query, State},
    http::{HeaderMap, Method},
};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use tokio::{process::Command, time::timeout};

use crate::{
    error::{AppError, AppResult},
    log_buffer,
    state::AppState,
};

use super::admin_auth;

const MAX_OUTPUT_BYTES: usize = 120_000;
const COMMAND_TIMEOUT_SECONDS: u64 = 15;
const POSTGRES_QUERY_MAX_ROWS: usize = 200;

#[derive(Deserialize)]
pub struct AdminDeveloperCliExecuteRequest {
    engine: String,
    command: String,
}

#[derive(Serialize)]
pub struct AdminDeveloperCliExecuteResponse {
    data: AdminDeveloperCliExecuteData,
}

#[derive(Serialize)]
struct AdminDeveloperCliExecuteData {
    engine: String,
    command: String,
    output: String,
    exit_code: i32,
    duration_ms: u64,
    truncated: bool,
}

#[derive(Deserialize)]
pub struct AdminDeveloperLogListQuery {
    limit: Option<usize>,
    keyword: Option<String>,
}

#[derive(Serialize)]
pub struct AdminDeveloperLogListResponse {
    data: Vec<String>,
    total: usize,
}

struct DeveloperCommandOutput {
    output: String,
    exit_code: i32,
    truncated: bool,
}

pub async fn admin_execute_developer_cli(
    State(state): State<AppState>,
    method: Method,
    headers: HeaderMap,
    Json(payload): Json<AdminDeveloperCliExecuteRequest>,
) -> AppResult<Json<AdminDeveloperCliExecuteResponse>> {
    let _principal = admin_auth::require_admin_principal(&state, &method, &headers)?;

    let engine = payload.engine.trim().to_lowercase();
    let command = payload.command.trim().to_string();
    if command.is_empty() {
        return Err(AppError::BadRequest("command is required".to_string()));
    }

    let started_at = Instant::now();
    let result = match engine.as_str() {
        "docker" => execute_docker_cli(&command).await?,
        "postgres" | "postgresql" => execute_postgresql_cli(&state, &command).await?,
        _ => {
            return Err(AppError::Unprocessable(
                "engine must be docker or postgresql".to_string(),
            ));
        }
    };
    let duration_ms = started_at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;

    Ok(Json(AdminDeveloperCliExecuteResponse {
        data: AdminDeveloperCliExecuteData {
            engine,
            command,
            output: result.output,
            exit_code: result.exit_code,
            duration_ms,
            truncated: result.truncated,
        },
    }))
}

pub async fn admin_list_developer_logs(
    State(state): State<AppState>,
    method: Method,
    headers: HeaderMap,
    Query(query): Query<AdminDeveloperLogListQuery>,
) -> AppResult<Json<AdminDeveloperLogListResponse>> {
    let _principal = admin_auth::require_admin_principal(&state, &method, &headers)?;
    let limit = query.limit.unwrap_or(300).clamp(1, 2_000);
    let keyword = query.keyword.as_deref();
    let (total, data) = log_buffer::list_recent_logs(limit, keyword);

    Ok(Json(AdminDeveloperLogListResponse { data, total }))
}

async fn execute_docker_cli(command_text: &str) -> AppResult<DeveloperCommandOutput> {
    let args = parse_cli_args(command_text)?;
    let normalized_args = normalize_docker_args(args)?;
    run_os_command(
        "docker",
        &normalized_args,
        Duration::from_secs(COMMAND_TIMEOUT_SECONDS),
    )
    .await
}

async fn execute_postgresql_cli(
    state: &AppState,
    command_text: &str,
) -> AppResult<DeveloperCommandOutput> {
    let trimmed = command_text.trim();
    let lower = trimmed.to_lowercase();

    if lower == "status" {
        let row = sqlx::query(
            "SELECT current_database() AS db_name, current_user AS db_user, now()::text AS now_at, version() AS pg_version",
        )
        .fetch_one(&state.db_pool)
        .await
        .map_err(|error| AppError::internal(format!("Failed to query PostgreSQL status: {error}")))?;

        let db_name = row
            .try_get::<String, _>("db_name")
            .unwrap_or_else(|_| "unknown".to_string());
        let db_user = row
            .try_get::<String, _>("db_user")
            .unwrap_or_else(|_| "unknown".to_string());
        let now_at = row
            .try_get::<String, _>("now_at")
            .unwrap_or_else(|_| "unknown".to_string());
        let pg_version = row
            .try_get::<String, _>("pg_version")
            .unwrap_or_else(|_| "unknown".to_string());

        return Ok(DeveloperCommandOutput {
            output: format!(
                "database : {db_name}\nuser     : {db_user}\nnow      : {now_at}\nversion  : {pg_version}"
            ),
            exit_code: 0,
            truncated: false,
        });
    }

    if lower == "databases" {
        let rows = sqlx::query(
            "SELECT datname AS name, pg_get_userbyid(datdba) AS owner \
             FROM pg_database WHERE datistemplate = false ORDER BY datname ASC",
        )
        .fetch_all(&state.db_pool)
        .await
        .map_err(|error| {
            AppError::internal(format!("Failed to list PostgreSQL databases: {error}"))
        })?;

        let table_rows = rows
            .into_iter()
            .map(|row| {
                vec![
                    row.try_get::<String, _>("name").unwrap_or_default(),
                    row.try_get::<String, _>("owner").unwrap_or_default(),
                ]
            })
            .collect::<Vec<Vec<String>>>();

        return Ok(DeveloperCommandOutput {
            output: render_table(&["database", "owner"], &table_rows),
            exit_code: 0,
            truncated: false,
        });
    }

    if lower == "tables" {
        let rows = sqlx::query(
            "SELECT tablename AS table_name FROM pg_tables \
             WHERE schemaname = 'public' ORDER BY tablename ASC",
        )
        .fetch_all(&state.db_pool)
        .await
        .map_err(|error| {
            AppError::internal(format!("Failed to list PostgreSQL tables: {error}"))
        })?;

        let table_rows = rows
            .into_iter()
            .map(|row| {
                vec![
                    row.try_get::<String, _>("table_name")
                        .unwrap_or_else(|_| "unknown".to_string()),
                ]
            })
            .collect::<Vec<Vec<String>>>();

        return Ok(DeveloperCommandOutput {
            output: render_table(&["table_name"], &table_rows),
            exit_code: 0,
            truncated: false,
        });
    }

    if lower.starts_with("query ") {
        let raw_sql = trimmed[6..].trim();
        let sql = normalize_read_only_sql(raw_sql)?;
        let wrapped_sql = format!(
            "SELECT row_to_json(result_row)::text AS row_json FROM ({sql}) AS result_row LIMIT {POSTGRES_QUERY_MAX_ROWS}"
        );
        let rows = sqlx::query(&wrapped_sql)
            .fetch_all(&state.db_pool)
            .await
            .map_err(|error| {
                AppError::internal(format!("Failed to execute PostgreSQL query: {error}"))
            })?;

        let output = if rows.is_empty() {
            "(0 rows)".to_string()
        } else {
            rows.into_iter()
                .enumerate()
                .map(|(index, row)| {
                    let line = row
                        .try_get::<String, _>("row_json")
                        .unwrap_or_else(|_| "{}".to_string());
                    format!("[{}] {line}", index + 1)
                })
                .collect::<Vec<String>>()
                .join("\n")
        };
        let (output, truncated) = truncate_output(&output, MAX_OUTPUT_BYTES);

        return Ok(DeveloperCommandOutput {
            output,
            exit_code: 0,
            truncated,
        });
    }

    Err(AppError::Unprocessable(
        "Unsupported PostgreSQL command. Use: status | databases | tables | query <SQL>"
            .to_string(),
    ))
}

fn normalize_read_only_sql(raw_sql: &str) -> AppResult<String> {
    let mut normalized = raw_sql.trim().to_string();
    if normalized.is_empty() {
        return Err(AppError::BadRequest(
            "SQL query cannot be empty".to_string(),
        ));
    }

    if normalized.ends_with(';') {
        normalized.pop();
    }
    normalized = normalized.trim().to_string();

    if normalized.is_empty() {
        return Err(AppError::BadRequest(
            "SQL query cannot be empty".to_string(),
        ));
    }
    if normalized.contains(';') {
        return Err(AppError::Forbidden(
            "Only a single read-only SQL statement is allowed".to_string(),
        ));
    }

    let upper = normalized.to_ascii_uppercase();
    if !(upper.starts_with("SELECT ")
        || upper.starts_with("SHOW ")
        || upper.starts_with("WITH ")
        || upper.starts_with("EXPLAIN "))
    {
        return Err(AppError::Forbidden(
            "Only read-only SQL is allowed (SELECT/SHOW/WITH/EXPLAIN)".to_string(),
        ));
    }

    let tokens = upper
        .split(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_')
        .filter(|token| !token.is_empty())
        .collect::<Vec<&str>>();
    let blocked = [
        "INSERT", "UPDATE", "DELETE", "DROP", "ALTER", "TRUNCATE", "CREATE", "GRANT", "REVOKE",
        "CALL", "DO", "VACUUM", "COPY",
    ];
    if tokens.iter().any(|token| blocked.contains(token)) {
        return Err(AppError::Forbidden(
            "Detected non-read-only SQL keyword".to_string(),
        ));
    }

    Ok(normalized)
}

fn parse_cli_args(input: &str) -> AppResult<Vec<String>> {
    let mut args = Vec::<String>::new();
    let mut current = String::new();
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut escaped = false;

    for ch in input.chars() {
        if escaped {
            current.push(ch);
            escaped = false;
            continue;
        }

        if ch == '\\' && !in_single_quote {
            escaped = true;
            continue;
        }

        if ch == '\'' && !in_double_quote {
            in_single_quote = !in_single_quote;
            continue;
        }
        if ch == '"' && !in_single_quote {
            in_double_quote = !in_double_quote;
            continue;
        }

        if ch.is_whitespace() && !in_single_quote && !in_double_quote {
            if !current.is_empty() {
                args.push(std::mem::take(&mut current));
            }
            continue;
        }

        current.push(ch);
    }

    if escaped || in_single_quote || in_double_quote {
        return Err(AppError::BadRequest(
            "Invalid command syntax: quote or escape is not closed".to_string(),
        ));
    }
    if !current.is_empty() {
        args.push(current);
    }
    if args.is_empty() {
        return Err(AppError::BadRequest("command is required".to_string()));
    }

    Ok(args)
}

fn normalize_docker_args(args: Vec<String>) -> AppResult<Vec<String>> {
    validate_cli_tokens(&args)?;
    let command = args[0].to_lowercase();

    match command.as_str() {
        "ps" | "images" | "version" | "info" => Ok(args),
        "inspect" => {
            if args.len() < 2 {
                return Err(AppError::BadRequest(
                    "docker inspect requires a target name".to_string(),
                ));
            }
            Ok(args)
        }
        "logs" => {
            if args.len() < 2 {
                return Err(AppError::BadRequest(
                    "docker logs requires a container name".to_string(),
                ));
            }
            let mut normalized = args;
            if !has_tail_flag(&normalized[1..]) {
                normalized.push("--tail".to_string());
                normalized.push("500".to_string());
            }
            Ok(normalized)
        }
        "compose" => normalize_docker_compose_args(args),
        _ => Err(AppError::Forbidden(
            "Unsupported docker command. Allowed: ps | images | version | info | inspect | logs | compose ps|logs".to_string(),
        )),
    }
}

fn normalize_docker_compose_args(args: Vec<String>) -> AppResult<Vec<String>> {
    if args.len() < 2 {
        return Err(AppError::BadRequest(
            "docker compose requires a subcommand".to_string(),
        ));
    }

    let subcommand = args[1].to_lowercase();
    if subcommand != "ps" && subcommand != "logs" {
        return Err(AppError::Forbidden(
            "Only docker compose ps/logs is allowed".to_string(),
        ));
    }

    let mut normalized = args;
    if subcommand == "logs" && !has_tail_flag(&normalized[2..]) {
        normalized.push("--tail".to_string());
        normalized.push("500".to_string());
    }
    Ok(normalized)
}

fn validate_cli_tokens(tokens: &[String]) -> AppResult<()> {
    if tokens.iter().any(|token| token.is_empty()) {
        return Err(AppError::BadRequest(
            "Invalid command syntax: empty argument".to_string(),
        ));
    }
    if tokens.iter().any(|token| token.len() > 256) {
        return Err(AppError::BadRequest(
            "Invalid command syntax: argument is too long".to_string(),
        ));
    }
    if tokens
        .iter()
        .any(|token| token.contains('\n') || token.contains('\r') || token.contains('\0'))
    {
        return Err(AppError::BadRequest(
            "Invalid command syntax: control characters are not allowed".to_string(),
        ));
    }

    Ok(())
}

fn has_tail_flag(args: &[String]) -> bool {
    args.iter()
        .any(|arg| arg == "--tail" || arg.starts_with("--tail="))
}

async fn run_os_command(
    program: &str,
    args: &[String],
    timeout_duration: Duration,
) -> AppResult<DeveloperCommandOutput> {
    let mut cmd = Command::new(program);
    cmd.args(args);
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    cmd.kill_on_drop(true);

    let output = timeout(timeout_duration, cmd.output())
        .await
        .map_err(|_| {
            AppError::ServiceUnavailable(format!(
                "Command timed out after {} seconds",
                timeout_duration.as_secs()
            ))
        })?
        .map_err(|error| {
            AppError::ServiceUnavailable(format!("Failed to execute `{program}`: {error}"))
        })?;

    let exit_code = output.status.code().unwrap_or(-1);
    let merged_output = merge_stdout_stderr(&output.stdout, &output.stderr);
    let (output, truncated) = truncate_output(&merged_output, MAX_OUTPUT_BYTES);

    Ok(DeveloperCommandOutput {
        output,
        exit_code,
        truncated,
    })
}

fn merge_stdout_stderr(stdout: &[u8], stderr: &[u8]) -> String {
    let mut text = String::new();
    if !stdout.is_empty() {
        text.push_str(String::from_utf8_lossy(stdout).as_ref());
    }
    if !stderr.is_empty() {
        if !text.is_empty() {
            text.push('\n');
        }
        text.push_str(String::from_utf8_lossy(stderr).as_ref());
    }

    if text.trim().is_empty() {
        "(no output)".to_string()
    } else {
        text.trim_end().to_string()
    }
}

fn truncate_output(text: &str, max_bytes: usize) -> (String, bool) {
    if text.len() <= max_bytes {
        return (text.to_string(), false);
    }

    let mut end = max_bytes;
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    let mut truncated = text[..end].trim_end().to_string();
    truncated.push_str("\n...(输出已截断)");
    (truncated, true)
}

fn render_table(headers: &[&str], rows: &[Vec<String>]) -> String {
    if rows.is_empty() {
        return "(0 rows)".to_string();
    }

    let mut widths = headers
        .iter()
        .map(|text| text.len())
        .collect::<Vec<usize>>();
    for row in rows {
        for (index, value) in row.iter().enumerate() {
            if index >= widths.len() {
                widths.push(value.len());
            } else if value.len() > widths[index] {
                widths[index] = value.len();
            }
        }
    }

    let header_line = headers
        .iter()
        .enumerate()
        .map(|(index, text)| format!("{text:width$}", width = widths[index]))
        .collect::<Vec<String>>()
        .join(" | ");
    let separator = widths
        .iter()
        .map(|width| "-".repeat(*width))
        .collect::<Vec<String>>()
        .join("-+-");
    let body = rows
        .iter()
        .map(|row| {
            row.iter()
                .enumerate()
                .map(|(index, value)| format!("{value:width$}", width = widths[index]))
                .collect::<Vec<String>>()
                .join(" | ")
        })
        .collect::<Vec<String>>()
        .join("\n");

    format!("{header_line}\n{separator}\n{body}")
}
