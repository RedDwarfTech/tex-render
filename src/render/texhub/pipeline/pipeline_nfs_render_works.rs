use crate::model::project::compile_app_params::CompileAppParams;
use crate::util::cv_util::copy_pdf_to_output_dir;
use log::error;
use rust_wheel::{
    common::util::rd_file_util::join_paths,
    config::app::app_conf_reader::get_app_config,
    texhub::{proj::compile_result::CompileResult, project::get_proj_path},
};
use std::{
    fs::{self, OpenOptions},
    io::{Error, Write},
    path::Path,
    process::Command,
};

// Recursively copy a directory's contents from `src` to `dst`.
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

fn tex_filename_from_path(path: &str) -> String {
    Path::new(path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(path)
        .to_string()
}

fn run_xelatex_in_dir(tex_file: &str, dir: &str) -> Result<std::process::Output, std::io::Error> {
    Command::new("xelatex")
        .arg(tex_file)
        .current_dir(dir)
        .output()
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

pub fn render_texhub_project_pipeline_nfs(params: &CompileAppParams) -> Option<CompileResult> {
    let proj_src_base_dir = get_app_config("cv.texhub_proj_base_dir");
    let texhub_output_dir = get_app_config("cv.texhub_proj_compile_base_dir");
    let proj_time_split_dir = get_proj_path(&proj_src_base_dir, params.proj_created_time);
    let proj_src_dir = join_paths(&[proj_time_split_dir.clone(), params.project_id.clone()]);
    // Prepare a separate compile directory under the configured compile base dir
    let time_split_output_proj_base = get_proj_path(&texhub_output_dir, params.proj_created_time);
    let compile_dir = join_paths(&[time_split_output_proj_base.clone(), params.project_id.clone()]);

    // Create compile directory and copy files
    let src_path = Path::new(&proj_src_dir);
    let dst_path = Path::new(&compile_dir);

    // Validate source directory exists
    if !src_path.exists() {
        error!(
            "source project dir not found: {}, params: {:?}",
            src_path.display(),
            params
        );
        return Some(CompileResult::Failure);
    }

    // Create target directory and its parents before copying
    if let Err(e) = fs::create_dir_all(dst_path) {
        error!(
            "failed to create compile directory: {}, dst: {}, params: {:?}",
            e,
            dst_path.display(),
            params
        );
        return Some(CompileResult::Failure);
    }

    // Copy files from source to compile directory
    if let Err(e) = copy_dir_all(src_path, dst_path) {
        error!(
            "failed to copy project to compile dir: {}, src: {}, dst: {}, params: {:?}",
            e,
            src_path.display(),
            dst_path.display(),
            params
        );
        return Some(CompileResult::Failure);
    }

    // Run xelatex in the compile directory using only the filename
    let tex_file_name = tex_filename_from_path(&params.file_path);
    let cmd = match run_xelatex_in_dir(&tex_file_name, &compile_dir) {
        Ok(o) => Ok(o),
        Err(e) => Err(e),
    };
    if let Err(e) = cmd {
        error!("compile tex file failed: {}, parmas: {:?}", e, params);
        return Some(CompileResult::Failure);
    }
    // write log into the compile directory so compile artifacts and logs are colocated
    let log_file_name = format!("{}/{}", compile_dir, params.log_file_name);
    let file: Result<std::fs::File, Error> = OpenOptions::new()
        .append(true)
        .create(true)
        .open(&log_file_name);
    if let Err(e) = file {
        error!(
            "open log file failed: {}, parmas: {:?}, log full path: {}",
            e, params, &log_file_name
        );
        return Some(CompileResult::Failure);
    }
    let mut naked_file = file.unwrap();
    write_end_marker(&mut naked_file, params);
    copy_pdf_to_output_dir(params, &compile_dir);
    return Some(CompileResult::Success);
}
