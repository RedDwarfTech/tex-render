use crate::controller::tex::tex_controller::update_queue_compile_result_sync;
use crate::rest::client::cv_client::{construct_headers, http_client_sync, http_client_sync_large_upload};
use crate::{
    model::project::compile_app_params::CompileAppParams, rest::client::cv_client::http_client,
};
use log::{error, info, warn};
use redis::{self, Connection};
use rust_wheel::{
    common::util::rd_file_util::join_paths,
    config::app::app_conf_reader::get_app_config,
    texhub::{proj::compile_result::CompileResult, project::get_proj_path},
};
use serde_json::json;
use std::io::{Read, Seek, SeekFrom, Write};
use std::{
    env,
    fs::{self, File, OpenOptions},
    io::Error,
    path::Path,
    process::{Command, Stdio},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::task;
use zip::read::ZipArchive;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

// Recursively copy a directory's contents from `src` to `dst`.
#[allow(dead_code)]
fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
    if !dst.exists() {
        fs::create_dir_all(dst)?;
    }
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let dest_path = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &dest_path)?;
        } else if ty.is_file() {
            fs::copy(&entry.path(), &dest_path)?;
        }
        // ignore symlinks and other types
    }
    Ok(())
}

#[allow(dead_code)]
fn tex_filename_from_path(path: &str) -> String {
    Path::new(path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(path)
        .to_string()
}

fn xelatex_log_path(compile_dir: &str, tex_file: &str) -> String {
    let base = Path::new(tex_file)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(tex_file);
    format!("{}/{}.log", compile_dir, base)
}

fn latexmk_log_path(compile_dir: &str) -> String {
    format!("{}/.latexmk.log", compile_dir)
}

fn end_marker_reached(end_marker_path: &str) -> bool {
    fs::read_to_string(end_marker_path)
        .map(|s| s.contains("====END===="))
        .unwrap_or(false)
}

fn read_xelatex_log(compile_dir: &str, tex_file: &str) -> Option<String> {
    let log_path = xelatex_log_path(compile_dir, tex_file);
    match fs::read_to_string(&log_path) {
        Ok(content) if !content.is_empty() => Some(content),
        Ok(_) => None,
        Err(e) => {
            warn!("Failed to read xelatex log at {}: {}", log_path, e);
            None
        }
    }
}

/// Remove stale latexmk artifacts so the next run always recompiles instead of
/// reporting "Nothing to do" with a cached previous error.
fn clean_latexmk_artifacts(compile_dir: &str, tex_file: &str) {
    let status = Command::new("latexmk")
        .arg("-C")
        .arg("-xelatex")
        .arg(tex_file)
        .current_dir(compile_dir)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    match status {
        Ok(s) if s.success() => {
            info!(
                "Cleaned latexmk artifacts before compile: compile_dir={}, tex_file={}",
                compile_dir, tex_file
            );
        }
        Ok(s) => {
            warn!(
                "latexmk -C exited with status {:?}: compile_dir={}, tex_file={}",
                s.code(),
                compile_dir,
                tex_file
            );
        }
        Err(e) => {
            warn!(
                "Failed to run latexmk -C before compile: compile_dir={}, tex_file={}, error={}",
                compile_dir, tex_file, e
            );
        }
    }
}

#[allow(dead_code)]
fn run_xelatex_in_dir(tex_file: &str, dir: &str) -> Result<std::process::Output, std::io::Error> {
    Command::new("xelatex")
        .arg(tex_file)
        .current_dir(dir)
        .output()
}

fn format_http_headers(headers: &reqwest::header::HeaderMap) -> String {
    if headers.is_empty() {
        return "(none)".to_string();
    }
    headers
        .iter()
        .map(|(name, value)| {
            format!(
                "{}={}",
                name,
                value.to_str().unwrap_or("<non-utf8>")
            )
        })
        .collect::<Vec<_>>()
        .join("; ")
}

fn truncate_for_log(content: &str, max_bytes: usize) -> String {
    if content.len() <= max_bytes {
        content.to_string()
    } else if max_bytes == 0 {
        format!("(truncated, total {} bytes)", content.len())
    } else {
        format!(
            "{}...(truncated, total {} bytes)",
            &content[..max_bytes],
            content.len()
        )
    }
}

fn format_http_send_error(
    method: &str,
    url: &str,
    request_body: &str,
    err: &reqwest::Error,
    timeout_secs: u64,
) -> String {
    let connect_timeout_secs = timeout_secs.min(10);
    let mut detail = format!(
        "HTTP request failed\n  method: {method}\n  url: {url}\n  request_headers: Content-Type=application/json\n  request_body: {request_body}\n  client_timeout: {timeout_secs}s\n  connect_timeout: {connect_timeout_secs}s\n  error: {err:#}\n  is_connect: {}\n  is_timeout: {}\n  is_request: {}\n  is_body: {}\n  is_decode: {}",
        err.is_connect(),
        err.is_timeout(),
        err.is_request(),
        err.is_body(),
        err.is_decode(),
    );
    if let Some(status) = err.status() {
        detail.push_str(&format!("\n  response_status: {status}"));
    }
    if err.is_connect() {
        detail.push_str(
            "\n  hint: connection failed — check DNS, target service, port, and network policy",
        );
    }
    if err.is_timeout() {
        detail.push_str(&format!(
            "\n  hint: request timed out (client_timeout={timeout_secs}s, connect_timeout={connect_timeout_secs}s)"
        ));
    }
    detail
}

fn format_http_error_response(
    method: &str,
    url: &str,
    request_body: &str,
    status: reqwest::StatusCode,
    resp_headers: &reqwest::header::HeaderMap,
    resp_body: &str,
) -> String {
    format!(
        "HTTP download failed\n  method: {method}\n  url: {url}\n  request_headers: Content-Type=application/json\n  request_body: {request_body}\n  response_status: {status}\n  response_headers: {}\n  response_body: {}",
        format_http_headers(resp_headers),
        truncate_for_log(resp_body, 2000),
    )
}

/**
 * Step 1: Download tex project source code zip package from texhub server.
 * Downloads from URL: /inner-tex/project/download/{project_id}
 * Returns path to the downloaded zip file.
 */
async fn download_tex_project_zip(
    project_id: &str,
    temp_dir: &str,
    x_request_id: &str,
) -> Result<String, String> {
    const DOWNLOAD_TIMEOUT_SECS: u64 = 30;
    let texhub_api_url = get_app_config("cv.texhub_api_url");
    let url = format!("{}/inner-tex/project/download", texhub_api_url);
    let zip_path = format!("{}/{}.zip", temp_dir, project_id);

    let body = json!({"project_id": project_id, "version": "latest"});
    let request_body = body.to_string();

    info!(
        "Downloading tex project zip: url={}, project_id={}, temp_dir={}, timeout={}s, x-request-id={}, request_body={}",
        url, project_id, temp_dir, DOWNLOAD_TIMEOUT_SECS, x_request_id, request_body
    );

    match http_client(Some(DOWNLOAD_TIMEOUT_SECS))
        .put(&url)
        .headers(construct_headers(Some(x_request_id)))
        .body(request_body.clone())
        .send()
        .await
    {
        Ok(resp) => {
            let status = resp.status();
            let resp_headers = resp.headers().clone();
            if !status.is_success() {
                let resp_body = resp
                    .text()
                    .await
                    .unwrap_or_else(|e| format!("<failed to read response body: {e:#}>"));
                let detail = format_http_error_response(
                    "PUT",
                    &url,
                    &request_body,
                    status,
                    &resp_headers,
                    &resp_body,
                );
                error!("{}", detail);
                return Err(detail);
            }
            match resp.bytes().await {
                Ok(bytes) => {
                    let byte_len = bytes.len();
                    match fs::write(&zip_path, bytes) {
                        Ok(_) => {
                            info!(
                                "Downloaded tex project zip to: {} ({} bytes)",
                                zip_path, byte_len
                            );
                            Ok(zip_path)
                        }
                        Err(e) => {
                            let detail = format!(
                                "Failed to write downloaded zip file\n  url: {url}\n  zip_path: {zip_path}\n  bytes_received: {byte_len}\n  error: {e:#}"
                            );
                            error!("{}", detail);
                            Err(detail)
                        }
                    }
                }
                Err(e) => {
                    let detail = format!(
                        "Failed to read download response body\n  url: {url}\n  response_status: {status}\n  response_headers: {}\n  error: {e:#}",
                        format_http_headers(&resp_headers),
                    );
                    error!("{}", detail);
                    Err(detail)
                }
            }
        }
        Err(e) => {
            let detail = format_http_send_error(
                "PUT",
                &url,
                &request_body,
                &e,
                DOWNLOAD_TIMEOUT_SECS,
            );
            error!("{}", detail);
            Err(detail)
        }
    }
}

/**
 * Step 2: Unzip the tex project to a specified directory using Rust zip library.
 * Uses the `zip` crate to extract zip files without relying on system unzip command.
 */
fn unzip_project(zip_path: &str, extract_dir: &str) -> Result<(), String> {
    info!(
        "Starting unzip operation using Rust zip library: zip_path={}, extract_dir={}",
        zip_path, extract_dir
    );

    // Open the zip file
    let zip_file = match File::open(zip_path) {
        Ok(file) => {
            info!("Successfully opened zip file: {}", zip_path);
            file
        }
        Err(e) => {
            error!(
                "Failed to open zip file: zip_path={}, error={}",
                zip_path, e
            );
            return Err(format!("Failed to open zip file: {}", e));
        }
    };

    // Create zip archive reader
    let mut archive = match ZipArchive::new(zip_file) {
        Ok(arch) => {
            info!(
                "Successfully created zip archive reader, entries: {}",
                arch.len()
            );
            arch
        }
        Err(e) => {
            error!(
                "Failed to create zip archive reader: zip_path={}, error={}",
                zip_path, e
            );
            return Err(format!("Failed to read zip archive: {}", e));
        }
    };

    // Ensure extract directory exists
    let extract_path = Path::new(extract_dir);
    if !extract_path.exists() {
        fs::create_dir_all(extract_path).map_err(|e| {
            error!(
                "Failed to create extract directory: extract_dir={}, error={}",
                extract_dir, e
            );
            format!("Failed to create extract directory: {}", e)
        })?;
        info!("Created extract directory: {}", extract_dir);
    }

    // Extract all files from the archive
    let total_entries = archive.len();
    let mut extracted_count = 0;
    let mut skipped_count = 0;
    let mut warning_count = 0;

    for i in 0..archive.len() {
        let mut file = match archive.by_index(i) {
            Ok(f) => f,
            Err(e) => {
                error!("Failed to read entry {} from zip: error={}", i, e);
                warning_count += 1;
                continue;
            }
        };

        // Sanitize the file path to prevent directory traversal attacks
        let outpath = match file.enclosed_name() {
            Some(path) => {
                // Strip absolute paths and normalize the path
                let path_str = path.to_string_lossy();
                // Remove leading slashes and drive letters (Windows)
                let cleaned = path_str
                    .trim_start_matches('/')
                    .trim_start_matches(|c: char| c.is_alphabetic() && c == ':')
                    .trim_start_matches('/');

                if cleaned.contains("..") {
                    warn!(
                        "Skipping potentially unsafe path in zip entry {}: {}",
                        i, path_str
                    );
                    skipped_count += 1;
                    continue;
                }

                extract_path.join(cleaned)
            }
            None => {
                // 绝对路径 fallback：去掉开头的 '/'，当作相对路径
                let raw = file.name();
                let cleaned = if raw.starts_with('/') {
                    raw.trim_start_matches('/').to_string()
                } else {
                    raw.to_string()
                };

                // 仍然检查是否包含 .. （安全起见）
                if cleaned.contains("..") {
                    warn!(
                        "Skipping entry {}: path contains '..' even after stripping: '{}'",
                        i, cleaned
                    );
                    skipped_count += 1;
                    continue;
                }
                extract_path.join(&cleaned)
            }
        };

        if file.name().ends_with('/') {
            fs::create_dir_all(&outpath).map_err(|e| {
                error!("Failed to create directory {:?}: error={}", outpath, e);
                format!("Failed to create directory {:?}: {}", outpath, e)
            })?;
        } else {
            if let Some(parent) = outpath.parent() {
                if !parent.exists() {
                    fs::create_dir_all(parent).map_err(|e| {
                        error!(
                            "Failed to create parent directory {:?}: error={}",
                            parent, e
                        );
                        format!("Failed to create parent directory: {}", e)
                    })?;
                }
            }

            // Extract file
            let mut outfile = File::create(&outpath).map_err(|e| {
                error!("Failed to create file {:?}: error={}", outpath, e);
                format!("Failed to create file {:?}: {}", outpath, e)
            })?;

            std::io::copy(&mut file, &mut outfile).map_err(|e| {
                error!("Failed to write file {:?}: error={}", outpath, e);
                format!("Failed to write file {:?}: {}", outpath, e)
            })?;

            extracted_count += 1;
        }
    }

    info!(
        "Unzip completed successfully. zip_path={}, extract_dir={}, total_entries={}, extracted={}, skipped={}, warnings={}",
        zip_path, extract_dir, total_entries, extracted_count, skipped_count, warning_count
    );

    if warning_count > 0 {
        warn!(
            "Completed with {} warnings (some entries may have been skipped)",
            warning_count
        );
    }

    Ok(())
}

/**
 * Step 3: Run latexmk (xelatex mode).
 * latexmk stdout/stderr goes to a local `.latexmk.log` (server-side only).
 * The xelatex `.log` file is tailed to Redis for the frontend.
 */
async fn run_latexmk_and_log(
    tex_file: &str,
    compile_dir: &str,
    end_marker_path: &str,
    params: &CompileAppParams,
) -> Result<(), String> {
    let latexmk_log_path = latexmk_log_path(compile_dir);
    info!(
        "Starting latexmk compilation (xelatex mode): tex_file={}, compile_dir={}, latexmk_log={}",
        tex_file, compile_dir, latexmk_log_path
    );

    clean_latexmk_artifacts(compile_dir, tex_file);

    let latexmk_log = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&latexmk_log_path)
        .map_err(|e| {
            error!(
                "Failed to open latexmk log file: {}, tex_file={}, compile_dir={}, params: {:?}",
                e, tex_file, compile_dir, params
            );
            format!("Failed to open latexmk log file: {}", e)
        })?;

    let latexmk_log_stderr = match latexmk_log.try_clone() {
        Ok(c) => c,
        Err(e) => {
            error!("Failed to clone latexmk log file handle: {}", e);
            return Err(format!("Failed to clone latexmk log file handle: {}", e));
        }
    };

    let mut child = match Command::new("latexmk")
        .arg("-xelatex")
        .arg("-interaction=nonstopmode")
        .arg("-synctex=1")
        .arg(tex_file)
        .current_dir(compile_dir)
        .stdout(Stdio::from(latexmk_log))
        .stderr(Stdio::from(latexmk_log_stderr))
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            error!(
                "Failed to start latexmk process: tex_file={}, compile_dir={}, error={}, params: {:?}",
                tex_file, compile_dir, e, params
            );
            let _ = open_write_end_marker(end_marker_path, params);
            return Err(format!("Failed to start latexmk process: {}", e));
        }
    };

    let status = match child.wait() {
        Ok(s) => s,
        Err(e) => {
            error!("Failed to wait on latexmk process: {}", e);
            let _ = open_write_end_marker(end_marker_path, params);
            return Err(format!("Failed to wait on latexmk process: {}", e));
        }
    };

    let exit_code = status
        .code()
        .map(|c| c.to_string())
        .unwrap_or_else(|| "unknown (terminated by signal)".to_string());

    if status.success() {
        handle_compile_success(tex_file, compile_dir, &exit_code, params, end_marker_path);
        Ok(())
    } else {
        error!(
            "latexmk compilation failed (xelatex mode): tex_file={}, compile_dir={}, exit_code={}",
            tex_file, compile_dir, exit_code
        );
        error!(
            "Compilation parameters: project_id={}, file_path={}, latexmk_log={}",
            params.project_id, params.file_path, latexmk_log_path
        );
        if let Ok(latexmk_output) = fs::read_to_string(&latexmk_log_path) {
            if !latexmk_output.is_empty() {
                error!(
                    "latexmk output (tail, server-side only):\n{}",
                    &latexmk_output
                        .chars()
                        .rev()
                        .take(1200)
                        .collect::<String>()
                        .chars()
                        .rev()
                        .collect::<String>()
                );
            }
        }
        let xelatex_log = read_xelatex_log(compile_dir, tex_file);
        let error_summary = extract_compilation_errors(xelatex_log.as_deref());
        if !error_summary.is_empty() {
            error!(
                "Key compilation errors detected:\n{}，xelatex log: {}",
                error_summary,
                xelatex_log_path(compile_dir, tex_file)
            );
        }
        let error_msg = format!(
            "latexmk compilation failed (exit code: {}). See xelatex log for details.",
            exit_code
        );
        do_upload_output_to_texhub(params, compile_dir, "pdf".to_owned());
        do_upload_output_to_texhub(params, compile_dir, "log".to_owned());
        do_upload_full_output_to_texhub(params, compile_dir);
        update_queue_compile_result_sync(params.clone(), Some(CompileResult::Failure));
        let _ = open_write_end_marker(end_marker_path, params);
        Err(error_msg)
    }
}

fn handle_compile_success(
    tex_file: &str,
    compile_dir: &str,
    exit_code: &str,
    params: &CompileAppParams,
    end_marker_path: &str,
) {
    info!(
        "xelatex compilation succeeded: tex_file={}, compile_dir={}, exit_code={}",
        tex_file, compile_dir, exit_code
    );
    update_queue_compile_result_sync(params.clone(), Some(CompileResult::Success));
    do_upload_output_to_texhub(params, compile_dir, "pdf".to_owned());
    do_upload_output_to_texhub(params, compile_dir, "log".to_owned());
    do_upload_full_output_to_texhub(params, compile_dir);
    let _ = open_write_end_marker(end_marker_path, params);
}

/// Extract key error messages from the xelatex log (server-side diagnostics).
fn extract_compilation_errors(xelatex_log: Option<&str>) -> String {
    let mut errors = Vec::new();

    if let Some(log) = xelatex_log {
        for line in log.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with('!') {
                errors.push(format!("LaTeX error: {}", trimmed));
            }
        }

        let error_patterns = vec![
            ("Fatal error", "Fatal errors"),
            ("Undefined control sequence", "Undefined control sequences"),
            ("Emergency stop", "Emergency stops"),
        ];
        for (pattern, label) in error_patterns {
            for line in log.lines() {
                if line.contains(pattern) {
                    errors.push(format!("{}: {}", label, line.trim()));
                }
            }
        }
    }

    if errors.is_empty() {
        if let Some(log) = xelatex_log {
            let lines: Vec<&str> = log.lines().collect();
            if !lines.is_empty() {
                let last_lines: Vec<&str> = lines.iter().rev().take(30).rev().cloned().collect();
                format!("Last output lines:\n{}", last_lines.join("\n"))
            } else {
                String::new()
            }
        } else {
            String::new()
        }
    } else {
        errors.join("\n")
    }
}

fn create_consumer_group(params: &CompileAppParams, con: &mut Connection) {
    // stream key namespaced by project id
    let stream_key = format!("texhub:compile:log:{}:{}", params.project_id, params.qid);
    let consumer_group = &params.project_id; // Use project_id as consumer group name

    // Create consumer group if it doesn't exist
    // XGROUP CREATE stream_key group_name $ MKSTREAM
    // Using MKSTREAM to create the stream if it doesn't exist
    let create_group_res: redis::RedisResult<()> = redis::cmd("XGROUP")
        .arg("CREATE")
        .arg(&stream_key)
        .arg(consumer_group)
        .arg("$")
        .arg("MKSTREAM")
        .query(con);

    match create_group_res {
        Ok(_) => {
            info!(
                "Created or verified consumer group '{}' for stream '{}'",
                consumer_group, stream_key
            );
        }
        Err(e) => {
            // If group already exists, that's fine (BUSYGROUP error)
            let err_str = e.to_string();
            if err_str.contains("BUSYGROUP") {
                info!(
                    "Consumer group '{}' already exists for stream '{}'",
                    consumer_group, stream_key
                );
            } else {
                error!(
                    "Failed to create consumer group '{}' for stream '{}': {}. Continuing anyway.",
                    consumer_group, stream_key, e
                );
            }
        }
    }
}

pub fn del_redis_stream(params: &CompileAppParams, con: &mut Connection) {
    let stream_key = format!("texhub:compile:log:{}:{}", params.project_id, params.qid);
    // Clear the stream before writing new logs
    let clear_res: redis::RedisResult<()> = redis::cmd("DEL").arg(&stream_key).query(con);

    if let Err(e) = clear_res {
        error!(
            "Failed to clear redis stream {}: {}. Continuing anyway.",
            stream_key, e
        );
    } else {
        info!("Cleared redis stream: {}", stream_key);
    }
}

/**
 * Step 4: Write compile log to redis stream (optional).
 * For now, we've already written to the local log file.
 * If redis integration is needed, uncomment and implement.
 */
fn write_log_to_redis_stream(log_content: &str, params: &CompileAppParams, con: &mut Connection) {
    let stream_key = format!("texhub:compile:log:{}:{}", params.project_id, params.qid);
    let line_count = log_content.lines().count();
    if line_count == 0 {
        return;
    }

    let mut pushed = 0u32;
    for line in log_content.lines() {
        let res: redis::RedisResult<String> = redis::cmd("XADD")
            .arg(&stream_key)
            .arg("MAXLEN")
            .arg("~")
            .arg(5000)
            .arg("*")
            .arg("msg")
            .arg(line)
            .query(con);

        match res {
            Ok(entry_id) => {
                pushed += 1;
                if pushed == 1 {
                    info!(
                        "Redis XADD started: stream={}, project_id={}, qid={}, first_entry_id={}",
                        stream_key, params.project_id, params.qid, entry_id
                    );
                }
            }
            Err(e) => {
                error!(
                    "Failed to XADD compile log to redis stream {}: {}. Line: {}",
                    stream_key, e, line
                );
            }
        }
    }

    info!(
        "Redis XADD batch done: stream={}, lines_pushed={}/{}, bytes={}",
        stream_key,
        pushed,
        line_count,
        log_content.len()
    );
}

fn build_multipart_upload_body(
    project_id: &str,
    file_name: &str,
    file_data: &[u8],
    mime_type: &str,
) -> (Vec<u8>, String) {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let boundary = format!("----rust-multipart-{}-{}", project_id, ts);
    let mut body: Vec<u8> = Vec::new();

    body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
    body.extend_from_slice(b"Content-Disposition: form-data; name=\"project_id\"\r\n\r\n");
    body.extend_from_slice(project_id.as_bytes());
    body.extend_from_slice(b"\r\n");

    body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
    body.extend_from_slice(
        format!(
            "Content-Disposition: form-data; name=\"file\"; filename=\"{}\"\r\n",
            file_name
        )
        .as_bytes(),
    );
    body.extend_from_slice(format!("Content-Type: {}\r\n\r\n", mime_type).as_bytes());
    body.extend_from_slice(file_data);
    body.extend_from_slice(b"\r\n");
    body.extend_from_slice(format!("--{}--\r\n", boundary).as_bytes());

    let content_type = format!("multipart/form-data; boundary={}", boundary);
    (body, content_type)
}

fn send_multipart_upload(
    upload_url: &str,
    body: Vec<u8>,
    content_type: String,
    upload_label: &str,
    x_request_id: &str,
) -> Result<(), String> {
    info!(
        "Uploading {} to texhub at URL: {} (multipart manual), x-request-id={}",
        upload_label, upload_url, x_request_id
    );
    match http_client_sync()
        .post(upload_url)
        .headers(construct_headers(Some(x_request_id)))
        .header("Content-Type", content_type)
        .body(body)
        .send()
    {
        Ok(resp) => {
            if resp.status().is_success() {
                Ok(())
            } else {
                let status = resp.status();
                let headers = resp.headers().clone();
                let body_text = match resp.text() {
                    Ok(t) => t,
                    Err(e) => format!("<failed to read body: {}>", e),
                };
                error!(
                    "{} upload failed. url: {} status: {} headers: {:?} body: {}",
                    upload_label, upload_url, status, headers, body_text
                );
                Err(format!("Upload failed with status: {}", status))
            }
        }
        Err(e) => {
            error!(
                "HTTP request to upload {} failed: {:#}, url: {}",
                upload_label, e, upload_url
            );
            Err(format!("HTTP request failed: {:#}", e))
        }
    }
}

fn write_multipart_upload_to_file(
    project_id: &str,
    file_name: &str,
    file_path: &str,
    mime_type: &str,
    multipart_path: &str,
) -> Result<String, String> {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let boundary = format!("----rust-multipart-{}-{}", project_id, ts);
    let mut out = File::create(multipart_path)
        .map_err(|e| format!("Failed to create multipart temp file: {}", e))?;

    out.write_all(format!("--{}\r\n", boundary).as_bytes())
        .map_err(|e| format!("Failed to write multipart header: {}", e))?;
    out.write_all(b"Content-Disposition: form-data; name=\"project_id\"\r\n\r\n")
        .map_err(|e| format!("Failed to write project_id field: {}", e))?;
    out.write_all(project_id.as_bytes())
        .map_err(|e| format!("Failed to write project_id value: {}", e))?;
    out.write_all(b"\r\n")
        .map_err(|e| format!("Failed to write multipart separator: {}", e))?;

    out.write_all(format!("--{}\r\n", boundary).as_bytes())
        .map_err(|e| format!("Failed to write file field header: {}", e))?;
    out.write_all(
        format!(
            "Content-Disposition: form-data; name=\"file\"; filename=\"{}\"\r\n",
            file_name
        )
        .as_bytes(),
    )
    .map_err(|e| format!("Failed to write file disposition: {}", e))?;
    out.write_all(format!("Content-Type: {}\r\n\r\n", mime_type).as_bytes())
        .map_err(|e| format!("Failed to write file content type: {}", e))?;

    let mut input = File::open(file_path)
        .map_err(|e| format!("Failed to open upload file {:?}: {}", file_path, e))?;
    std::io::copy(&mut input, &mut out)
        .map_err(|e| format!("Failed to stream file into multipart body: {}", e))?;

    out.write_all(b"\r\n")
        .map_err(|e| format!("Failed to write multipart trailing separator: {}", e))?;
    out.write_all(format!("--{}--\r\n", boundary).as_bytes())
        .map_err(|e| format!("Failed to write multipart closing boundary: {}", e))?;
    out.sync_all()
        .map_err(|e| format!("Failed to sync multipart temp file: {}", e))?;

    Ok(format!("multipart/form-data; boundary={}", boundary))
}

fn send_multipart_upload_from_file(
    upload_url: &str,
    multipart_path: &str,
    content_type: String,
    upload_label: &str,
    x_request_id: &str,
) -> Result<(), String> {
    let multipart_size = fs::metadata(multipart_path)
        .map(|m| m.len())
        .unwrap_or(0);
    info!(
        "Uploading {} ({} bytes) to texhub at URL: {} (multipart stream), x-request-id={}",
        upload_label, multipart_size, upload_url, x_request_id
    );

    let file = File::open(multipart_path)
        .map_err(|e| format!("Failed to open multipart temp file: {}", e))?;

    match http_client_sync_large_upload()
        .post(upload_url)
        .headers(construct_headers(Some(x_request_id)))
        .header("Content-Type", content_type)
        .body(file)
        .send()
    {
        Ok(resp) => {
            if resp.status().is_success() {
                Ok(())
            } else {
                let status = resp.status();
                let headers = resp.headers().clone();
                let body_text = match resp.text() {
                    Ok(t) => t,
                    Err(e) => format!("<failed to read body: {}>", e),
                };
                error!(
                    "{} upload failed. url: {} status: {} headers: {:?} body: {}",
                    upload_label, upload_url, status, headers, body_text
                );
                Err(format!("Upload failed with status: {}", status))
            }
        }
        Err(e) => {
            error!(
                "HTTP request to upload {} failed: {:#}, url: {}, multipart_size: {}",
                upload_label, e, upload_url, multipart_size
            );
            Err(format!("HTTP request failed: {:#}", e))
        }
    }
}

/**
 * Step 5: Upload the compiled PDF file to texhub server via HTTP.
 * Uses multipart form data or binary upload.
 */
fn upload_file_to_texhub(file_path: &str, project_id: &str, x_request_id: &str) -> Result<(), String> {
    let texhub_api_url = get_app_config("cv.texhub_api_url");
    let upload_url = format!("{}/inner-tex/project/upload-output", texhub_api_url);

    let file_data = fs::read(file_path).map_err(|e| format!("Failed to read PDF file: {}", e))?;

    let file_name = Path::new(file_path)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "output.pdf".to_string());

    let (body, content_type) =
        build_multipart_upload_body(project_id, &file_name, &file_data, "application/pdf");
    send_multipart_upload(&upload_url, body, content_type, "PDF", x_request_id)
}

fn zip_directory(src_dir: &str, zip_path: &str) -> Result<(), String> {
    let src_path = Path::new(src_dir);
    if !src_path.exists() {
        return Err(format!("Directory not found: {}", src_dir));
    }

    let zip_file =
        File::create(zip_path).map_err(|e| format!("Failed to create zip file: {}", e))?;
    let mut zip = ZipWriter::new(zip_file);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

    fn add_path_to_zip(
        zip: &mut ZipWriter<File>,
        path: &Path,
        base: &Path,
        options: SimpleFileOptions,
    ) -> Result<(), String> {
        if path.is_dir() {
            for entry in fs::read_dir(path)
                .map_err(|e| format!("Failed to read dir {:?}: {}", path, e))?
            {
                let entry = entry.map_err(|e| format!("Failed to read dir entry: {}", e))?;
                add_path_to_zip(zip, &entry.path(), base, options)?;
            }
            Ok(())
        } else if path.is_file() {
            let relative = path
                .strip_prefix(base)
                .map_err(|e| format!("Failed to get relative path for {:?}: {}", path, e))?;
            let name = relative.to_string_lossy().replace('\\', "/");
            zip.start_file(name.clone(), options)
                .map_err(|e| format!("Failed to start zip entry {}: {}", name, e))?;
            let mut file =
                File::open(path).map_err(|e| format!("Failed to open file {:?}: {}", path, e))?;
            std::io::copy(&mut file, zip)
                .map_err(|e| format!("Failed to write file {:?} to zip: {}", path, e))?;
            Ok(())
        } else {
            Ok(())
        }
    }

    add_path_to_zip(&mut zip, src_path, src_path, options)?;
    zip.finish()
        .map_err(|e| format!("Failed to finalize zip file: {}", e))?;
    Ok(())
}

/**
 * Step 6: Upload the full compile output directory as a zip archive to texhub server.
 */
fn upload_full_output_to_texhub(
    file_path: &str,
    project_id: &str,
    x_request_id: &str,
) -> Result<(), String> {
    let texhub_api_url = get_app_config("cv.texhub_api_url");
    let upload_url = format!("{}/inner-tex/project/upload-full-output", texhub_api_url);

    let zip_size = fs::metadata(file_path).map(|m| m.len()).unwrap_or(0);
    info!(
        "Full output zip ready for upload: path={}, size={} bytes",
        file_path, zip_size
    );

    let file_name = Path::new(file_path)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "full-output.zip".to_string());

    let multipart_path = format!("/tmp/texhub_full_output_multipart_{}.tmp", project_id);
    let content_type = write_multipart_upload_to_file(
        project_id,
        &file_name,
        file_path,
        "application/zip",
        &multipart_path,
    )?;

    let result = send_multipart_upload_from_file(
        &upload_url,
        &multipart_path,
        content_type,
        "full output",
        x_request_id,
    );
    let _ = fs::remove_file(&multipart_path);
    result
}

fn write_end_marker(file: &mut std::fs::File, params: &CompileAppParams) {
    let wr = file.write_all("====END====\n".as_bytes());
    if let Err(e) = wr {
        error!("write log file failed: {}, parmas: {:?}", e, params);
    }
    let sync_result = file.sync_all();
    if let Err(e) = sync_result {
        error!("sync log file failed: {}, parmas: {:?}", e, params);
    }
}

/*
 * step 1: download the tex project source code zip package by http from texhub server
 * the url path: /inner-tex/project/download/{project_id}
 * step 2: unzip the tex project to a temp folder
 * step 3: run latexmk (xelatex) to compile the tex file
 * step 4: stream xelatex .log to redis (latexmk log stays server-side only)
 * step 5: upload the compiled pdf file to texhub server by http
 * step 6: zip compile output and upload to /inner-tex/project/upload-full-output
 */
pub fn render_texhub_project_pipeline(params: &CompileAppParams) -> Option<CompileResult> {
    // compute compile and log paths
    let texhub_output_dir = get_app_config("cv.texhub_proj_compile_base_dir");
    let time_split_output_proj_base = get_proj_path(&texhub_output_dir, params.proj_created_time);
    let compile_dir = join_paths(&[
        time_split_output_proj_base.clone(),
        params.project_id.clone(),
    ]);
    let log_file_path = format!("{}/{}", compile_dir, params.log_file_name);
    let tex_file_name = tex_filename_from_path(&params.file_path);
    let xelatex_log_path = xelatex_log_path(&compile_dir, &tex_file_name);

    // ensure compile dir
    if let Err(e) = ensure_compile_dir(&compile_dir, params) {
        error!("ensure compile dir failed: {}", e);
        return Some(CompileResult::Failure);
    }

    // download & unzip
    if let Err(e) = download_and_unzip(params, &compile_dir, &time_split_output_proj_base) {
        error!(
            "download/unzip failed: project_id={}, qid={}, compile_dir={}, file_path={}, detail:\n{}",
            params.project_id, params.qid, compile_dir, params.file_path, e
        );
        return Some(CompileResult::Failure);
    }
    let params_copy = params.clone();
    let compile_dir_copy = compile_dir.clone();
    let end_marker_path_copy = log_file_path.clone();
    // end marker file only; xelatex log is produced by the compiler
    let _ = fs::write(&log_file_path, "");
    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| format!("create runtime failed: {}", e))
        .unwrap();
    task::spawn_blocking(move || {
        if let Err(e) = rt.block_on(compile_project(
            &params_copy,
            &compile_dir_copy,
            &end_marker_path_copy,
        )) {
            error!("compile step failed: {}", e);
        }
    });
    if let Err(e) = tail_xelatex_log(params, &xelatex_log_path, &log_file_path) {
        error!("xelatex log tail failed: {}", e);
    }
    Some(CompileResult::Success)
}

// --- Small helpers to keep pipeline readable ---

fn ensure_compile_dir(compile_dir: &str, params: &CompileAppParams) -> Result<(), String> {
    let p = Path::new(compile_dir);
    if !p.exists() {
        fs::create_dir_all(p).map_err(|e| format!("create compile dir failed: {}", e))?;
    }
    Ok(())
}

fn download_and_unzip(
    params: &CompileAppParams,
    compile_dir: &str,
    unzip_dir: &str,
) -> Result<(), String> {
    let temp_dir = format!("/tmp/texhub_downloads_{}", params.project_id);
    info!(
        "download_and_unzip start: project_id={}, qid={}, compile_dir={}, unzip_dir={}, temp_dir={}",
        params.project_id, params.qid, compile_dir, unzip_dir, temp_dir
    );

    if let Err(e) = fs::create_dir_all(&temp_dir) {
        let detail = format!(
            "create temp dir failed\n  temp_dir: {temp_dir}\n  project_id: {}\n  error: {e:#}",
            params.project_id
        );
        error!("{}", detail);
        return Err(detail);
    }

    let rt = tokio::runtime::Runtime::new().map_err(|e| format!("create runtime failed: {}", e))?;
    let zip_path = match rt.block_on(download_tex_project_zip(
        &params.project_id,
        &temp_dir,
        &params.x_request_id,
    )) {
        Ok(path) => path,
        Err(e) => {
            error!(
                "download_tex_project_zip failed: project_id={}, qid={}, temp_dir={}, detail:\n{}",
                params.project_id, params.qid, temp_dir, e
            );
            let _ = fs::remove_dir_all(&temp_dir);
            return Err(format!("download failed: {}", e));
        }
    };

    info!(
        "About to unzip file: zip_path={}, unzip_dir={}, project_id={}",
        zip_path, unzip_dir, params.project_id
    );
    match unzip_project(&zip_path, &unzip_dir) {
        Ok(_) => {
            info!(
                "Unzip completed successfully: project_id={}, zip_path={}, unzip_dir={}",
                params.project_id, zip_path, unzip_dir
            );
            let _ = fs::remove_file(&zip_path);
            let _ = fs::remove_dir_all(&temp_dir);
            Ok(())
        }
        Err(e) => {
            error!(
                "Unzip failed: project_id={}, qid={}, zip_path={}, unzip_dir={}, detail:\n{}",
                params.project_id, params.qid, zip_path, unzip_dir, e
            );
            let _ = fs::remove_file(&zip_path);
            let _ = fs::remove_dir_all(&temp_dir);
            Err(format!("unzip failed: {}", e))
        }
    }
}

async fn compile_project(
    params: &CompileAppParams,
    compile_dir: &str,
    end_marker_path: &str,
) -> Result<(), String> {
    let tex_file_name = tex_filename_from_path(&params.file_path);
    return run_latexmk_and_log(&tex_file_name, &compile_dir, end_marker_path, params).await;
}

fn open_write_end_marker(end_marker_path: &str, params: &CompileAppParams) -> Result<(), String> {
    // write end marker
    let file: Result<File, Error> = OpenOptions::new()
        .append(true)
        .create(true)
        .open(end_marker_path);
    let mut naked_file = file.map_err(|e| format!("open log failed: {}", e))?;
    info!(
        "Writing compile end marker: path={}, project_id={}, qid={}",
        end_marker_path, params.project_id, params.qid
    );
    write_end_marker(&mut naked_file, params);
    drop(naked_file);
    return Ok(());
}

fn do_upload_output_to_texhub(params: &CompileAppParams, compile_dir: &str, extension: String) {
    // upload pdf (best-effort)
    let pdf_file_name = format!(
        "{}.{}",
        params
            .file_path
            .split('.')
            .next()
            .unwrap_or(&params.file_path),
        extension
    );
    let pdf_path = format!(
        "{}/{}",
        compile_dir,
        Path::new(&pdf_file_name)
            .file_name()
            .unwrap()
            .to_string_lossy()
    );
    info!("Uploading compiled output from path: {}", pdf_path);
    if Path::new(&pdf_path).exists() {
        let _ = upload_file_to_texhub(&pdf_path, &params.project_id, &params.x_request_id);
    } else {
        warn!("Compiled output not found at: {}", pdf_path);
    }
}

fn do_upload_full_output_to_texhub(params: &CompileAppParams, compile_dir: &str) {
    let zip_path = format!("/tmp/texhub_full_output_{}.zip", params.project_id);
    info!(
        "Packaging compile output for upload: compile_dir={}, zip_path={}",
        compile_dir, zip_path
    );
    if let Err(e) = zip_directory(compile_dir, &zip_path) {
        error!("Failed to zip compile output: {}", e);
        return;
    }
    match upload_full_output_to_texhub(&zip_path, &params.project_id, &params.x_request_id) {
        Ok(_) => info!(
            "Full output upload succeeded for project: {}",
            params.project_id
        ),
        Err(e) => error!(
            "Full output upload failed for project {}: {}",
            params.project_id, e
        ),
    }
    let _ = fs::remove_file(&zip_path);
}

fn tail_xelatex_log(
    params: &CompileAppParams,
    xelatex_log_path: &str,
    end_marker_path: &str,
) -> Result<(), String> {
    let redis_url = env::var("REDIS_URL").unwrap();
    let client = redis::Client::open(redis_url.as_str()).map_err(|e| {
        error!(
            "Failed to create redis client for url {}: {}. Logging locally.",
            redis_url, e
        );
        format!("Redis client creation failed: {}", e)
    })?;

    let mut con = client.get_connection().map_err(|e| {
        error!("Failed to get redis connection: {}. Logging locally.", e);
        format!("Redis connection failed: {}", e)
    })?;

    del_redis_stream(params, &mut con);
    create_consumer_group(params, &mut con);

    info!(
        "Starting log push tail: project_id={}, qid={}, stream=texhub:compile:log:{}:{}, xelatex_log={}, end_marker={}",
        params.project_id,
        params.qid,
        params.project_id,
        params.qid,
        xelatex_log_path,
        end_marker_path
    );

    let mut pos: u64 = 0;
    let mut log_file_seen = false;
    let mut push_rounds: u32 = 0;
    let poll_interval = Duration::from_millis(200);
    let max_wait = Duration::from_secs(3600);
    let started = std::time::Instant::now();

    loop {
        if Path::new(xelatex_log_path).exists() {
            if !log_file_seen {
                log_file_seen = true;
                let file_size = fs::metadata(xelatex_log_path).map(|m| m.len()).unwrap_or(0);
                info!(
                    "xelatex log file appeared: path={}, initial_size={} bytes",
                    xelatex_log_path, file_size
                );
            }
            let prev_pos = pos;
            push_xelatex_log_delta(xelatex_log_path, &mut pos, params, &mut con);
            if pos > prev_pos {
                push_rounds += 1;
            }
        }

        if end_marker_reached(end_marker_path) {
            if Path::new(xelatex_log_path).exists() {
                let prev_pos = pos;
                push_xelatex_log_delta(xelatex_log_path, &mut pos, params, &mut con);
                if pos > prev_pos {
                    push_rounds += 1;
                }
            }
            if !log_file_seen {
                warn!(
                    "End marker reached but xelatex log never appeared: path={}",
                    xelatex_log_path
                );
            }
            info!(
                "Log push tail finished: project_id={}, qid={}, bytes_sent={}, push_rounds={}, elapsed={:?}, log_seen={}",
                params.project_id,
                params.qid,
                pos,
                push_rounds,
                started.elapsed(),
                log_file_seen
            );
            break;
        }

        if started.elapsed() > max_wait {
            warn!(
                "xelatex log tail timed out after {:?}: path={}, bytes_sent={}, push_rounds={}, log_seen={}",
                max_wait, xelatex_log_path, pos, push_rounds, log_file_seen
            );
            break;
        }

        thread::sleep(poll_interval);
    }

    Ok(())
}

fn push_xelatex_log_delta(
    xelatex_log_path: &str,
    pos: &mut u64,
    params: &CompileAppParams,
    con: &mut Connection,
) {
    let start_pos = *pos;
    let mut f = match File::open(xelatex_log_path) {
        Ok(f) => f,
        Err(e) => {
            error!("read xelatex log failed: path={}, error={}", xelatex_log_path, e);
            return;
        }
    };
    if f.seek(SeekFrom::Start(*pos)).is_err() {
        warn!(
            "xelatex log seek failed: path={}, offset={}",
            xelatex_log_path, start_pos
        );
        return;
    }
    let mut contents = String::new();
    if f.read_to_string(&mut contents).is_err() {
        warn!(
            "xelatex log read failed: path={}, offset={}",
            xelatex_log_path, start_pos
        );
        return;
    }
    *pos = f.metadata().map(|m| m.len()).unwrap_or(*pos);
    if !contents.is_empty() {
        info!(
            "xelatex log delta: path={}, offset {} -> {} ({} bytes, {} lines)",
            xelatex_log_path,
            start_pos,
            *pos,
            contents.len(),
            contents.lines().count()
        );
        write_log_to_redis_stream(&contents, params, con);
    }
}
