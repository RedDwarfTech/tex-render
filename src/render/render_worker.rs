use crate::{
    controller::tex::tex_controller::update_queue_compile_status,
    model::{
        cv::{cv_gen::CvGen, cv_main::CvMainResp},
        project::{
            compile_app_params::CompileAppParams, tex_file_compile_status::TeXFileCompileStatus,
        },
        request::cv::render_handle_request::RenderHandleRequest,
        response::tex::compile_output::CompileOutput,
        template::cv_template::CvTemplate,
    },
    rest::client::cv_client::update_gen_result
};
use chrono::{Datelike, Utc};
use log::{error, info, warn};
use rust_wheel::{
    common::util::{
        net::sse_message::SSEMessage,
        rd_file_util::{create_folder_not_exists, get_filename_without_ext, join_paths},
    },
    config::app::app_conf_reader::get_app_config,
    texhub::{proj::compile_result::CompileResult, project::get_proj_path},
};
use sha256::try_digest;
use std::{
    env,
    fs::{self, OpenOptions},
    io::{self, BufRead, BufReader, Error, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};
use tokio::{sync::mpsc::UnboundedSender, task};
use uuid::Uuid;

use super::cv::{
    dyweb::dyweb_handler::DywebHandler, handler::template_handler::TemplateHandler,
    hijiangtao::hijiangtao_handler::HijiangtaoHandler, moderncv::moderncv_handler::ModerncvHandler,
    moderncv1::moderncv_handler1::ModerncvHandler1, weitian::weitian_handler::WeitianHandler,
    zheyuye::zheyuye_handler::ZheyuyeHandler,
};

pub async fn render_texhub_project_sse(
    parmas: &CompileAppParams,
    tx: UnboundedSender<String>,
) -> io::Result<String> {
    let uuid = Uuid::new_v4();
    let uuid_string = uuid.to_string().replace("-", "");
    let folder_path = Path::new(&parmas.file_path)
        .parent()
        .unwrap()
        .to_string_lossy();
    let compile_out_path = format!("{}/{}", folder_path, uuid_string);
    create_folder_not_exists(&compile_out_path);
    let mut cmd = Command::new("xelatex")
        .arg("-output-directory")
        .arg(compile_out_path)
        .arg(parmas.file_path.clone())
        .stdout(Stdio::piped())
        .spawn()?;
    let stdout = cmd.stdout.take().unwrap();
    let reader = BufReader::new(stdout);
    task::spawn_blocking({
        let tx: UnboundedSender<String> = tx.clone();
        move || {
            let shared_tx = Arc::new(Mutex::new(tx));
            for line in reader.lines() {
                if let Ok(line) = line {
                    let msg_content = format!("{}\n", line.to_owned());
                    let sse_msg: SSEMessage<String> =
                        SSEMessage::from_data(msg_content.to_string(), &"TEX_COMP_LOG".to_string());
                    let sse_string = serde_json::to_string(&sse_msg);
                    let send_result = shared_tx.lock().unwrap().send(sse_string.unwrap());
                    if let Err(se) = send_result {
                        error!("send xelatex render compile log error: {}", se);
                    }
                }
            }
            do_msg_send(
                &"ok".to_string(),
                shared_tx,
                &"TEX_COMP_LOG_END".to_string(),
            );
        }
    });
    let status = cmd.wait()?;
    if status.success() {
        Ok("Compilation successful".to_string())
    } else {
        Err(io::Error::new(io::ErrorKind::Other, "Compilation failed"))
    }
}

pub fn render_texhub_project_mq(params: &CompileAppParams) -> Option<CompileResult> {
    let base_texhub_dir = get_app_config("cv.texhub_proj_base_dir");
    let proj_comp_dir = get_proj_path(&base_texhub_dir, params.proj_created_time);
    let current_dir = join_paths(&[proj_comp_dir.clone(), params.project_id.clone()]);
    // let compile_out_path = format!("{}/{}", current_dir, params.version_no);
    // create_folder_not_exists(&compile_out_path);
    // we remove the -output-directory because:
    // 1. facing this issue: https://tex.stackexchange.com/questions/697033/is-it-possible-to-auto-create-dist-folder-when-not-exists-using-xelatex-compile
    // 2. maybe output-directory have some compatible issue with latex compile engine
    let cmd = Command::new("xelatex")
        .arg("-synctex=1")
        //.arg("-output-directory")
        //.arg(compile_out_path.clone())
        .arg(params.file_path.clone())
        .current_dir(&current_dir)
        .output();
    if let Err(e) = cmd {
        error!("compile tex file failed: {}, parmas: {:?}", e, params);
        return Some(CompileResult::Failure);
    }
    let log_file_name = format!("{}/{}", current_dir, params.log_file_name);
    let file: Result<std::fs::File, Error> = OpenOptions::new().append(true).open(&log_file_name);
    if let Err(e) = file {
        error!(
            "open log file failed: {}, parmas: {:?}, log full path: {}",
            e, params, &log_file_name
        );
        return Some(CompileResult::Failure);
    }
    // flush newest compile status before the disk log end flag
    // avoid query to compiling status when send end flag to the client
    // reqwest requires a multi-threaded runtime to properly handle HTTP requests
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(update_queue_compile_status(
        params.to_owned(),
        TeXFileCompileStatus::Compiling,
    ));
    let mut naked_file = file.unwrap();
    let wr = naked_file.write_all("\n====END====\n".as_bytes());
    if let Err(e) = wr {
        error!("write log file failed: {}, parmas: {:?}", e, params);
    }
    let sync_result = naked_file.sync_all();
    if let Err(e) = sync_result {
        error!("sync log file failed: {}, parmas: {:?}", e, params);
    }
    let output = cmd.unwrap();
    let status = output.status;
    match status.code() {
        Some(code) => {
            if code != 0 {
                let stderr = String::from_utf8_lossy(&output.stderr);
                let stdout = String::from_utf8_lossy(&output.stdout);
                error!(
                    "Process exited with status code: {}. stderr: {}, stdout: {}",
                    code, stderr, stdout
                );
            }
        }
        None => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            warn!("Process terminated by signal. stderr: {}, stdout: {}", stderr, stdout);
        }
    }
    return Some(CompileResult::Success);
}


pub fn do_msg_send(
    line: &String,
    tx: Arc<std::sync::Mutex<UnboundedSender<String>>>,
    msg_type: &str,
) {
    let sse_msg: SSEMessage<String> =
        SSEMessage::from_data(line.to_string(), &msg_type.to_string());
    let sse_string = serde_json::to_string(&sse_msg);
    let send_result = tx.lock().unwrap().send(sse_string.unwrap());
    thread::sleep(Duration::from_secs(1));
    match send_result {
        Ok(_) => {}
        Err(e) => {
            error!("send xelatex compile log error: {}", e);
        }
    }
}

pub async fn render_texhub_project(parmas: &CompileAppParams) -> Option<CompileOutput> {
    let folder_path = Path::new(&parmas.file_path)
        .parent()
        .unwrap()
        .to_string_lossy();
    let compile_out_path = format!("{}/{}", folder_path, parmas.version_no);
    create_folder_not_exists(&compile_out_path);
    let output = Command::new("xelatex")
        .arg("-output-directory")
        .arg(compile_out_path)
        .arg(parmas.file_path.clone())
        .output();
    match output {
        Ok(out) => {
            if !out.status.success() {
                error!(
                    "latex compile doc failed, error: {}, file path: {}",
                    String::from_utf8(out.stderr).unwrap(),
                    parmas.file_path.clone()
                );
                return None;
            }
            warn!(
                "compile the doc success,out:{}, error: {}, file path: {}",
                String::from_utf8(out.stdout).unwrap(),
                String::from_utf8(out.stderr).unwrap(),
                parmas.file_path.clone()
            );
            let resp = CompileOutput {
                project_id: parmas.project_id.clone(),
                out_path: parmas.version_no.clone(),
                req_time: parmas.req_time.clone(),
            };
            return Some(resp);
        }
        Err(e) => {
            error!(
                "project xelatex command failed, {},file_path:{},out_path:{}",
                e, parmas.file_path, parmas.out_path
            );
            return None;
        }
    }
}

pub async fn render_impl(cv_gen: &CvGen, cv_tpl: CvTemplate, cv_main: CvMainResp) {
    let relative_path = get_relative_path(cv_gen.user_id, cv_gen.template_id, cv_gen.cv_id);
    let out_path = get_dist_path(&relative_path);
    let result = fs::create_dir_all(&out_path);
    let file_path = format!("{}{}", out_path, "/modern.tex");
    let handler = ModerncvHandler {
        next: Some(Box::new(ZheyuyeHandler {
            next: Some(Box::new(DywebHandler {
                next: Some(Box::new(WeitianHandler {
                    next: Some(Box::new(HijiangtaoHandler {
                        next: Some(Box::new(ModerncvHandler1 {})),
                    })),
                })),
            })),
        })),
    };
    let req = RenderHandleRequest {
        template_code: cv_tpl.template_code.unwrap(),
        file_path: &file_path,
        cv_main: cv_main.clone(),
    };
    handler.handle_request(req, &cv_main).unwrap();
    if result.is_ok() {
        let output = Command::new("xelatex")
            .arg("-output-directory")
            .arg(&out_path)
            .arg(&file_path)
            .output();
        match output {
            Ok(succ_output) => {
                if succ_output.status.success() {
                    let file_name = copy_file_to_server(&file_path, &relative_path, "pdf").await;
                    let tex_file_name =
                        copy_file_to_server(&file_path, &relative_path, "tex").await;
                    update_gen_result(cv_gen.id, &file_name, &tex_file_name).await;
                    info!("Compilation successful!");
                } else {
                    let err_msg = String::from_utf8_lossy(&succ_output.stderr);
                    let out_msg = String::from_utf8_lossy(&succ_output.stdout);
                    error!(
                        "Compilation failed: std error: {}, std out: {}",
                        err_msg, out_msg
                    );
                }
            }
            Err(e) => {
                error!("execute xelatex command failed, {}", e);
            }
        }
    }
}

async fn copy_file_to_server(
    input_file_path: &str,
    out_relative_path: &str,
    file_type: &str,
) -> String {
    let file_path = PathBuf::from(input_file_path);
    let new_path = file_path.with_extension(file_type);
    let new_file_path = new_path.as_path().to_str().unwrap();
    let file_info = Path::new(new_file_path);
    let file_sha = try_digest(file_info).unwrap();
    let ssh_pwd = env::var("CV_REMOTE_SSH_PWD").unwrap();
    let output = Command::new("sshpass")
        .arg("-p")
        .arg(ssh_pwd)
        .arg("rsync")
        .arg("-avz")
        .arg("--mkpath") //https://stackoverflow.com/questions/1636889/how-can-i-configure-rsync-to-create-target-directory-on-remote-server
        .arg("--ignore-existing")
        .arg("-e")
        .arg("ssh")
        .arg(new_file_path)
        .arg(format!(
            "{}{}{}{}{}{}",
            "root@172.29.217.209:/data/k8s/reddwarf-pro/cv-server-service/cv/pdf/",
            out_relative_path,
            "/",
            file_sha,
            ".",
            file_type
        ))
        .output();
    match output {
        Ok(op) => {
            if op.status.success() {
                let pdf_file_name =
                    format!("{}/{}{}{}", out_relative_path, file_sha, ".", file_type);
                return pdf_file_name;
            } else {
                let error_msg = String::from_utf8_lossy(&op.stderr);
                error!(
                    "output facing error: {}, file path: {}",
                    error_msg, input_file_path
                );
                return "".to_string();
            }
        }
        Err(e) => {
            error!(
                "Failed to execute rsync command,error:{},input file path:{}, new file path:{}",
                e, input_file_path, new_file_path
            );
            return "".to_string();
        }
    }
}

fn get_dist_path(relative_path: &String) -> String {
    let base_cv_dir = get_app_config("cv.cv_compile_base_dir");
    let full_path = format!("{}{}{}", base_cv_dir, "/", relative_path);
    return full_path;
}

fn get_relative_path(user_id: i64, tpl_id: i64, cv_id: i64) -> String {
    let now = Utc::now();
    let year = now.year();
    let month = now.month();
    let time_path = format!("{}{}{}", year, "/", month);
    let user_path = format!("{}{}{}{}{}", user_id, "/", cv_id, "/", tpl_id);
    let relative_path = format!("{}{}{}{}", "/", time_path, "/", user_path);
    return relative_path;
}
