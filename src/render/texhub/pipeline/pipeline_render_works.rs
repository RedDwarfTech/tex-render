use crate::controller::tex::tex_controller::update_queue_compile_result_sync;
use crate::rest::client::cv_client::http_client_sync;
use crate::{
    model::project::compile_app_params::CompileAppParams, rest::client::cv_client::http_client,
};
use log::{error, info, warn};
use notify::RecursiveMode;
use notify::{Event, Watcher};
use redis::{self, Connection};
use rust_wheel::{
    common::util::rd_file_util::join_paths,
    config::app::app_conf_reader::get_app_config,
    texhub::{proj::compile_result::CompileResult, project::get_proj_path},
};
use serde_json::json;
use std::io::{Read, Seek, SeekFrom, Write};
use std::sync::mpsc;
use std::{
    env,
    fs::{self, File, OpenOptions},
    io::Error,
    path::Path,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::task;
use zip::read::ZipArchive;

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

#[allow(dead_code)]
fn run_xelatex_in_dir(tex_file: &str, dir: &str) -> Result<std::process::Output, std::io::Error> {
    Command::new("xelatex")
        .arg(tex_file)
        .current_dir(dir)
        .output()
}

/**
 * Step 1: Download tex project source code zip package from texhub server.
 * Downloads from URL: /inner-tex/project/download/{project_id}
 * Returns path to the downloaded zip file.
 */
async fn download_tex_project_zip(project_id: &str, temp_dir: &str) -> Result<String, String> {
    let texhub_api_url = get_app_config("cv.texhub_api_url");
    let url = format!("{}/inner-tex/project/download", texhub_api_url);
    let zip_path = format!("{}/{}.zip", temp_dir, project_id);

    let body = json!({"project_id": project_id, "version": "latest"});

    match http_client().put(&url).json(&body).send().await {
        Ok(resp) => {
            if !resp.status().is_success() {
                return Err(format!(
                    "Download failed with status: {}, url: {}",
                    resp.status(),
                    url
                ));
            }
            match resp.bytes().await {
                Ok(bytes) => match fs::write(&zip_path, bytes) {
                    Ok(_) => {
                        info!("Downloaded tex project zip to: {}", zip_path);
                        Ok(zip_path)
                    }
                    Err(e) => Err(format!("Failed to write zip file: {}", e)),
                },
                Err(e) => Err(format!("Failed to read response body: {}", e)),
            }
        }
        Err(e) => Err(format!("HTTP request failed: {}", e)),
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
 * Step 3 (enhanced): Run xelatex and capture stdout/stderr to a log file.
 */
async fn run_xelatex_and_log(
    tex_file: &str,
    compile_dir: &str,
    log_file_path: &str,
    params: &CompileAppParams,
) -> Result<(), String> {
    info!(
        "Starting xelatex compilation: tex_file={}, compile_dir={}, log_file={}",
        tex_file, compile_dir, log_file_path
    );

    let cmd = Command::new("xelatex")
        .arg("-interaction=nonstopmode")
        .arg("-synctex=1")
        .arg(tex_file)
        .current_dir(compile_dir)
        .output();

    if let Err(e) = cmd {
        error!(
            "Failed to start xelatex process: tex_file={}, compile_dir={}, error={}, params: {:?}",
            tex_file, compile_dir, e, params
        );
        return Err(format!("Failed to start xelatex process: {}", e));
    }

    let output = cmd.unwrap();
    let status = output.status;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let exit_code = status
        .code()
        .map(|c| c.to_string())
        .unwrap_or_else(|| "unknown (terminated by signal)".to_string());

    if status.success() {
        info!(
            "xelatex compilation succeeded: tex_file={}, compile_dir={}, exit_code={}",
            tex_file, compile_dir, exit_code
        );
        if !stdout.is_empty() {
            let stdout_len = stdout.len();
            let preview = if stdout_len > 500 {
                &stdout[stdout_len.saturating_sub(500)..]
            } else {
                &stdout
            };
        }
        update_queue_compile_result_sync(params.clone(), Some(CompileResult::Success));
        do_upload_pdf_to_texhub(params, compile_dir);
        let _ = open_write_end_marker(log_file_path, params);
        Ok(())
    } else {
        // Compilation failed - output detailed error information
        error!(
            "xelatex compilation failed: tex_file={}, compile_dir={}, exit_code={}",
            tex_file, compile_dir, exit_code
        );
        error!(
            "Compilation parameters: project_id={}, file_path={}, log_file={}",
            params.project_id, params.file_path, log_file_path
        );

        // Log stderr content (usually contains error messages)
        if !stderr.is_empty() {
            error!(
                "xelatex stderr (full output, {} bytes):\n{}",
                stderr.len(),
                stderr
            );
        }

        // Try to extract key error information from the output
        let error_summary = extract_compilation_errors(&stdout, &stderr);
        if !error_summary.is_empty() {
            error!("Key compilation errors detected:\n{}", error_summary);
        }

        // Write error details to log file
        if let Err(e) = write_compilation_errors_to_log(
            log_file_path,
            &stdout,
            &stderr,
            exit_code.as_str(),
            params,
        ) {
            warn!("Failed to write compilation errors to log file: {}", e);
        }

        let error_msg = format!(
            "xelatex compilation failed (exit code: {}). stdout_len={}, stderr_len={}. Check logs for details.",
            exit_code, stdout.len(), stderr.len()
        );

        update_queue_compile_result_sync(params.clone(), Some(CompileResult::Failure));
        let _ = open_write_end_marker(log_file_path, params);
        Err(error_msg)
    }
}

/// Extract key error messages from xelatex output
fn extract_compilation_errors(stdout: &str, stderr: &str) -> String {
    let mut errors = Vec::new();
    let combined = format!("{}\n{}", stdout, stderr);

    // Look for common LaTeX error patterns
    let error_patterns = vec![
        ("Error:", "LaTeX errors"),
        ("Fatal error", "Fatal errors"),
        ("Undefined control sequence", "Undefined control sequences"),
        ("Missing", "Missing items"),
        ("File not found", "File not found errors"),
        ("Emergency stop", "Emergency stops"),
    ];

    for (pattern, label) in error_patterns {
        for line in combined.lines() {
            if line.contains(pattern) {
                errors.push(format!("{}: {}", label, line.trim()));
            }
        }
    }

    if errors.is_empty() {
        // If no specific patterns found, return last few lines that might contain errors
        let lines: Vec<&str> = combined.lines().collect();
        if lines.len() > 0 {
            let last_lines: Vec<&str> = lines.iter().rev().take(10).rev().cloned().collect();
            format!("Last output lines:\n{}", last_lines.join("\n"))
        } else {
            String::new()
        }
    } else {
        errors.join("\n")
    }
}

/// Write compilation errors to the log file
fn write_compilation_errors_to_log(
    log_file_path: &str,
    stdout: &str,
    stderr: &str,
    exit_code: &str,
    params: &CompileAppParams,
) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(log_file_path)
        .map_err(|e| format!("Failed to open log file: {}", e))?;

    writeln!(file, "\n==== COMPILATION FAILED ====")
        .map_err(|e| format!("Failed to write to log: {}", e))?;
    writeln!(file, "Exit code: {}", exit_code)
        .map_err(|e| format!("Failed to write to log: {}", e))?;
    writeln!(file, "Project ID: {}", params.project_id)
        .map_err(|e| format!("Failed to write to log: {}", e))?;
    writeln!(file, "File path: {}", params.file_path)
        .map_err(|e| format!("Failed to write to log: {}", e))?;

    if !stdout.is_empty() {
        writeln!(file, "\n--- STDOUT ---").map_err(|e| format!("Failed to write to log: {}", e))?;
        file.write_all(stdout.as_bytes())
            .map_err(|e| format!("Failed to write stdout to log: {}", e))?;
    }

    if !stderr.is_empty() {
        writeln!(file, "\n--- STDERR ---").map_err(|e| format!("Failed to write to log: {}", e))?;
        file.write_all(stderr.as_bytes())
            .map_err(|e| format!("Failed to write stderr to log: {}", e))?;
    }

    writeln!(file, "\n==== END COMPILATION ERROR ====")
        .map_err(|e| format!("Failed to write to log: {}", e))?;
    file.sync_all()
        .map_err(|e| format!("Failed to sync log file: {}", e))?;

    Ok(())
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
    // Split content into lines and push each as an entry into the stream.
    // Each entry contains fields: msg, file, project_id, proj_created_time
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

        if let Err(e) = res {
            error!(
                "Failed to XADD compile log to redis stream {}: {}. Line: {}",
                stream_key, e, line
            );
        }
    }
}

/**
 * Step 5: Upload the compiled PDF file to texhub server via HTTP.
 * Uses multipart form data or binary upload.
 */
fn upload_file_to_texhub(file_path: &str, project_id: &str) -> Result<(), String> {
    let texhub_api_url = get_app_config("cv.texhub_api_url");
    let upload_url = format!("{}/inner-tex/project/upload-output", texhub_api_url);

    let file_data = fs::read(file_path).map_err(|e| format!("Failed to read PDF file: {}", e))?;

    // Manually build multipart/form-data body to avoid requiring reqwest multipart feature.
    let file_name = Path::new(file_path)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "output.pdf".to_string());

    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let boundary = format!("----rust-multipart-{}-{}", project_id, ts);
    let mut body: Vec<u8> = Vec::new();

    // project_id field
    body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
    body.extend_from_slice(b"Content-Disposition: form-data; name=\"project_id\"\r\n\r\n");
    body.extend_from_slice(project_id.as_bytes());
    body.extend_from_slice(b"\r\n");

    // file field
    body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
    body.extend_from_slice(
        format!(
            "Content-Disposition: form-data; name=\"file\"; filename=\"{}\"\r\n",
            file_name
        )
        .as_bytes(),
    );
    body.extend_from_slice(b"Content-Type: application/pdf\r\n\r\n");
    body.extend_from_slice(&file_data);
    body.extend_from_slice(b"\r\n");

    // final boundary
    body.extend_from_slice(format!("--{}--\r\n", boundary).as_bytes());

    let content_type = format!("multipart/form-data; boundary={}", boundary);
    info!(
        "Uploading PDF to texhub at URL: {} (multipart manual)",
        upload_url
    );
    match http_client_sync()
        .post(&upload_url)
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
                    "PDF upload failed. url: {} status: {} headers: {:?} body: {}",
                    upload_url, status, headers, body_text
                );
                Err(format!("Upload failed with status: {}", status))
            }
        }
        Err(e) => {
            error!(
                "HTTP request to upload PDF failed: {}, url: {}",
                e, upload_url
            );
            Err(format!("HTTP request failed: {}", e))
        }
    }
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
 * step 3: run xelatex command to compile the tex file
 * step 4: write compile log file to redis stream
 * step 5: upload the compiled pdf file to texhub server by http
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

    // ensure compile dir
    if let Err(e) = ensure_compile_dir(&compile_dir, params) {
        error!("ensure compile dir failed: {}", e);
        return Some(CompileResult::Failure);
    }

    // download & unzip
    if let Err(e) = download_and_unzip(params, &compile_dir, &time_split_output_proj_base) {
        error!("download/unzip failed: {}", e);
        return Some(CompileResult::Failure);
    }
    let params_copy = params.clone();
    let compile_dir_copy = compile_dir.clone();
    let log_file_path_copy = log_file_path.clone();
    // clear the log file contents before compilation
    // check if exists, if not create
    let _ = fs::write(&log_file_path, "");
    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| format!("create runtime failed: {}", e))
        .unwrap();
    task::spawn_blocking(move || {
        if let Err(e) = rt.block_on(compile_project(
            &params_copy,
            &compile_dir_copy,
            &log_file_path_copy,
        )) {
            error!("compile step failed: {}", e);
        }
    });
    if let Err(e) = tail_log(params, &log_file_path) {
        error!("finalize/upload failed: {}", e);
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
    // temp dir for download
    let temp_dir = format!("/tmp/texhub_downloads_{}", params.project_id);
    fs::create_dir_all(&temp_dir).map_err(|e| format!("create temp dir failed: {}", e))?;

    let rt = tokio::runtime::Runtime::new().map_err(|e| format!("create runtime failed: {}", e))?;
    let zip_path = rt.block_on(download_tex_project_zip(&params.project_id, &temp_dir))?;

    // unzip into compile_dir
    info!(
        "About to unzip file: zip_path={}, unzip_dir={}",
        zip_path, unzip_dir
    );
    match unzip_project(&zip_path, &unzip_dir) {
        Ok(_) => {
            info!("Unzip completed successfully, cleaning up temp files");
            let _ = fs::remove_file(&zip_path);
            let _ = fs::remove_dir_all(&temp_dir);
            Ok(())
        }
        Err(e) => {
            error!("Unzip failed: {}, cleaning up temp files", e);
            let _ = fs::remove_file(&zip_path);
            let _ = fs::remove_dir_all(&temp_dir);
            Err(format!("unzip failed: {}", e))
        }
    }
}

async fn compile_project(
    params: &CompileAppParams,
    compile_dir: &str,
    log_file_path: &str,
) -> Result<(), String> {
    let tex_file_name = tex_filename_from_path(&params.file_path);
    return run_xelatex_and_log(&tex_file_name, &compile_dir, log_file_path, params).await;
}

fn open_write_end_marker(log_file_path: &str, params: &CompileAppParams) -> Result<(), String> {
    // write end marker
    let file: Result<File, Error> = OpenOptions::new()
        .append(true)
        .create(true)
        .open(&log_file_path);
    let mut naked_file = file.map_err(|e| format!("open log failed: {}", e))?;
    info!("write end marker...");
    write_end_marker(&mut naked_file, params);
    drop(naked_file);
    return Ok(());
}

fn do_upload_pdf_to_texhub(params: &CompileAppParams, compile_dir: &str) {
    // upload pdf (best-effort)
    let pdf_file_name = format!(
        "{}.pdf",
        params
            .file_path
            .split('.')
            .next()
            .unwrap_or(&params.file_path)
    );
    let pdf_path = format!(
        "{}/{}",
        compile_dir,
        Path::new(&pdf_file_name)
            .file_name()
            .unwrap()
            .to_string_lossy()
    );
    info!("Uploading compiled PDF from path: {}", pdf_path);
    if Path::new(&pdf_path).exists() {
        let _ = upload_file_to_texhub(&pdf_path, &params.project_id);
    } else {
        warn!("Compiled PDF not found at: {}", pdf_path);
    }
}

fn tail_log(params: &CompileAppParams, log_file_path: &str) -> notify::Result<()> {
    // Create Redis client and connection once, reuse for all log writes
    let redis_url = env::var("REDIS_URL").unwrap();
    let client = match redis::Client::open(redis_url.as_str()) {
        Ok(c) => c,
        Err(e) => {
            error!(
                "Failed to create redis client for url {}: {}. Logging locally.",
                redis_url, e
            );
            return Err(notify::Error::generic(&format!(
                "Redis client creation failed: {}",
                e
            )));
        }
    };

    let mut con = match client.get_connection() {
        Ok(c) => c,
        Err(e) => {
            error!("Failed to get redis connection: {}. Logging locally.", e);
            return Err(notify::Error::generic(&format!(
                "Redis connection failed: {}",
                e
            )));
        }
    };
    del_redis_stream(&params, &mut con);
    create_consumer_group(&params, &mut con);

    let (tx, rx) = mpsc::channel::<notify::Result<Event>>();
    // Use recommended_watcher() to automatically select the best implementation
    // for your platform. The `EventHandler` passed to this constructor can be a
    // closure, a `std::sync::mpsc::Sender`, a `crossbeam_channel::Sender`, or
    // another type the trait is implemented for.
    let mut watcher = notify::recommended_watcher(tx)?;
    // Add a path to be watched. All files and directories at that path and
    // below will be monitored for changes.
    watcher.watch(Path::new(log_file_path), RecursiveMode::Recursive)?;
    let mut contents = fs::read_to_string(&log_file_path).unwrap();
    let mut pos = contents.len() as u64;
    // Block forever, printing out events as they come in
    for res in rx {
        match res {
            Ok(event) => {
                let mut f = File::open(&log_file_path).unwrap();
                f.seek(SeekFrom::Start(pos)).unwrap();
                pos = f.metadata().unwrap().len();
                contents.clear();
                let read_result = f.read_to_string(&mut contents);
                if let Err(e) = read_result {
                    error!("read log file failed: {}", e);
                    continue;
                }
                write_log_to_redis_stream(&contents, params, &mut con);
                if contents.contains("====END====") {
                    info!("Detected end marker in log, stopping tail.");
                    break;
                }
            }
            Err(e) => error!("watch error: {:?}", e),
        }
    }
    drop(watcher);
    Ok(())
}
