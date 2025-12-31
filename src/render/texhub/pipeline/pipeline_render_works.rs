use crate::controller::tex::tex_controller::{update_queue_compile_result, update_queue_compile_result_sync};
use crate::util::cv_util::copy_pdf_to_output_dir;
use crate::{
    model::project::compile_app_params::CompileAppParams, rest::client::cv_client::http_client,
};
use log::{error, info, warn};
use notify::RecursiveMode;
use notify::{Event, Watcher};
use redis::{self, Connection};
use reqwest::Body;
use reqwest::Client;
use rust_wheel::{
    common::util::rd_file_util::join_paths,
    config::app::app_conf_reader::get_app_config,
    texhub::{proj::compile_result::CompileResult, project::get_proj_path},
};
use serde_json::json;
use std::io::{Read, Seek, SeekFrom};
use std::sync::mpsc;
use std::{
    env,
    fs::{self, File, OpenOptions},
    io::{Error, Write},
    path::Path,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::task;

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
 * Step 2: Unzip the tex project to a specified directory.
 * For now, use the `unzip` system command (requires `unzip` to be installed).
 * Alternatively, could use the `zip` crate if available.
 */
#[allow(dead_code)]
fn unzip_project(zip_path: &str, extract_dir: &str) -> Result<(), String> {
    let output = Command::new("unzip")
        .arg("-o")
        .arg(zip_path)
        .arg("-d")
        .arg(extract_dir)
        .output();

    match output {
        Ok(out) => {
            if out.status.success() {
                info!("Unzipped project to: {}", extract_dir);
                Ok(())
            } else {
                let stderr = String::from_utf8_lossy(&out.stderr);
                Err(format!("unzip command failed: {}", stderr))
            }
        }
        Err(e) => Err(format!("Failed to run unzip command: {}", e)),
    }
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
    let cmd = Command::new("xelatex")
        .arg(tex_file)
        .current_dir(compile_dir)
        .output();

    if let Err(e) = cmd {
        error!("compile tex file failed: {}, parmas: {:?}", e, "params");
        return Err(" Failed to start xelatex process".to_string());
    }
    let output = cmd.unwrap();
    let status = output.status;
    if status.success() {
        info!("xelatex compilation succeeded");
        // copy pdf to local output
        let pdf_path = copy_pdf_to_output_dir(params, &compile_dir.to_string());
        update_queue_compile_result_sync(params.clone(), Some(CompileResult::Success));

        let project_id = params.project_id.clone();
        do_upload_pdf_to_texhub(&pdf_path, &project_id, params, compile_dir);
        let _ = open_write_end_marker(log_file_path, params);
        Ok(())
    } else {
        let exit_code = status
            .code()
            .map(|c| c.to_string())
            .unwrap_or_else(|| "unknown".to_string());
        let msg = format!("xelatex exited with code: {}", exit_code);
        error!("{}", msg);
        update_queue_compile_result_sync(params.clone(), Some(CompileResult::Failure));
        let _ = open_write_end_marker(log_file_path, params);
        Err(msg)
    }
}

fn create_consumer_group(params: &CompileAppParams, con: &mut Connection) {
    // stream key namespaced by project id
    let stream_key = format!("texhub:compile:log:{}", params.project_id);
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

fn del_redis_stream(params: &CompileAppParams, con: &mut Connection) {
    let stream_key = format!("texhub:compile:log:{}", params.project_id);
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
    let stream_key = format!("texhub:compile:log:{}", params.project_id);
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
async fn upload_pdf_to_texhub(pdf_path: &str, project_id: &str) -> Result<(), String> {
    let texhub_api_url = get_app_config("cv.texhub_api_url");
    let upload_url = format!("{}/inner-tex/project/upload-output", texhub_api_url);

    let file_data = fs::read(pdf_path).map_err(|e| format!("Failed to read PDF file: {}", e))?;

    // Manually build multipart/form-data body to avoid requiring reqwest multipart feature.
    let file_name = Path::new(pdf_path)
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
    match Client::new()
        .post(&upload_url)
        .header("Content-Type", content_type)
        .body(Body::from(body))
        .send()
        .await
    {
        Ok(resp) => {
            if resp.status().is_success() {
                info!("Successfully uploaded PDF to texhub: {}", upload_url);
                Ok(())
            } else {
                let status = resp.status();
                let headers = resp.headers().clone();
                let body_text = match resp.text().await {
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
    task::spawn_blocking(move || {
        // Create a multi-threaded runtime inside spawn_blocking to run async compile_project
        // reqwest requires a multi-threaded runtime to properly handle HTTP requests
        let rt = tokio::runtime::Runtime::new().map_err(|e| format!("create runtime failed: {}", e)).unwrap();
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
    unzip_project(&zip_path, &unzip_dir).map_err(|e| format!("unzip failed: {}", e))?;
    let _ = fs::remove_file(&zip_path);
    let _ = fs::remove_dir_all(&temp_dir);
    Ok(())
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

fn do_upload_pdf_to_texhub(
    pdf_path: &str,
    project_id: &str,
    params: &CompileAppParams,
    compile_dir: &str,
) {
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
        upload_pdf_to_texhub(&pdf_path, &params.project_id);
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
                f.read_to_string(&mut contents).unwrap();
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
