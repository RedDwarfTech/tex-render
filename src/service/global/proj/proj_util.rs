use rust_wheel::{
    common::util::{rd_file_util::join_paths, time_util::get_current_millisecond},
    config::app::app_conf_reader::get_app_config,
    texhub::project::get_proj_path,
};

pub fn get_proj_base_dir(proj_id: &String, created_time: i64) -> String {
    let base_compile_dir: String = get_app_config("cv.texhub_proj_compile_base_dir");
    let proj_base_dir = get_proj_path(&base_compile_dir, created_time);
    let proj_dir = join_paths(&[proj_base_dir, proj_id.to_owned()]);
    return proj_dir;
}
