use rust_wheel::{
    common::util::{rd_file_util::join_paths, time_util::get_current_millisecond},
    config::app::app_conf_reader::get_app_config,
    texhub::project::get_proj_path,
};

pub fn get_proj_base_dir(proj_id: &String, created_time: i64) -> String {
    let base_compile_dir: String = get_app_config("texhub.texhub_proj_compile_base_dir");
    let proj_base_dir = get_proj_path(&base_compile_dir, created_time);
    let proj_dir = join_paths(&[proj_base_dir, proj_id.to_owned()]);
    return proj_dir;
}

pub fn get_purge_proj_base_dir(proj_id: &String, created_time: i64) -> String {
    let base_compile_dir: String = get_app_config("texhub.compile_base_dir");
    let proj_base_dir = get_proj_path(&base_compile_dir, created_time);
    let proj_dir = join_paths(&[proj_base_dir, proj_id.to_owned()]);
    return proj_dir;
}

pub fn get_proj_download_base_dir(proj_id: &String, created_time: i64) -> String {
    let base_compile_dir: String = get_app_config("texhub.download_base_dir");
    let proj_base_dir = get_proj_path(&base_compile_dir, created_time);
    let proj_dir = join_paths(&[proj_base_dir, proj_id.to_owned()]);
    return proj_dir;
}

// because we create the project in transaction
// when get the project from database, the transaction is not commmitted
// so we have to get the folder instantly, it only works with create the project
// this method only works for the invoke moment
// do not use it to get the history project work directory
pub fn get_proj_base_dir_instant(proj_id: &String) -> String {
    let base_compile_dir: String = get_app_config("texhub.compile_base_dir");
    let ct = get_current_millisecond();
    let proj_base_dir = get_proj_path(&base_compile_dir, ct);
    let proj_dir = join_paths(&[proj_base_dir, proj_id.to_owned()]);
    return proj_dir;
}
