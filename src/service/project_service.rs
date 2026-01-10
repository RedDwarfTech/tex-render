use std::{
    ffi::{c_int, CStr, CString},
    path::PathBuf,
};

use crate::{
    common::interop::synctex::{
        synctex_display_query, synctex_edit_query, synctex_node_box_visible_depth,
        synctex_node_box_visible_h, synctex_node_box_visible_height, synctex_node_box_visible_v,
        synctex_node_box_visible_width, synctex_node_column, synctex_node_line, synctex_node_p,
        synctex_node_page, synctex_node_tag, synctex_node_visible_h, synctex_node_visible_v,
        synctex_scanner_free, synctex_scanner_get_name, synctex_scanner_new_with_output_file,
        synctex_scanner_next_result,
    },
    model::{
        request::proj::{get_pdf_pos_params::GetPdfPosParams, get_src_pos_params::GetSrcPosParams},
        response::proj::{pdf_pos_resp::PdfPosResp, src_pos_resp::SrcPosResp},
    },
    service::global::proj::proj_util::get_proj_base_dir,
};
use log::{error, info};
use rust_wheel::common::util::rd_file_util::{get_filename_without_ext, join_paths};
use std::path::Path;

pub fn get_pdf_pos(params: &GetPdfPosParams) -> Vec<PdfPosResp> {
    info!("get pdf pos params:{:?}", params);
    let proj_dir = get_proj_base_dir(&params.project_id, params.create_time);
    let pdf_file_name = format!("{}{}", get_filename_without_ext(&params.main_file), ".pdf");
    let full_pdf_file_path = join_paths(&[&proj_dir, &pdf_file_name.to_string()]);
    unsafe {
        let c_pdf_full_file_path = CString::new(full_pdf_file_path.clone());
        info!("full pdf path:{}", full_pdf_file_path.clone());
        if let Err(e) = c_pdf_full_file_path {
            error!("parse out path error,{},{}", e, full_pdf_file_path.clone());
            return Vec::new();
        }
        let c_build_path = CString::new(proj_dir.clone());
        if let Err(e) = c_build_path {
            error!("parse build path error,{},{}", e, proj_dir.clone());
            return Vec::new();
        }
        let cstring_pdf_full_file_path = c_pdf_full_file_path.unwrap();
        let cstring_build_path = c_build_path.unwrap();
        let scanner = synctex_scanner_new_with_output_file(
            cstring_pdf_full_file_path.as_ptr(),
            cstring_build_path.as_ptr(),
            1,
        );
        let tex_file_path = join_paths(&[proj_dir, params.path.clone(), params.file.clone()]);
        let demo_tex = CString::new(tex_file_path.clone());
        let mut position_list: Vec<PdfPosResp> = Vec::new();
        let cstring_demo_tex = demo_tex.unwrap();
        let node_number = synctex_display_query(
            scanner,
            cstring_demo_tex.as_ptr(),
            params.line as c_int,
            params.column as c_int,
            0,
        );
        if node_number > 0 {
            for _i in 0..node_number {
                let node: synctex_node_p = synctex_scanner_next_result(scanner);
                let page = synctex_node_page(node);
                // this code was inspired from synctex synctex main viewer procceed code
                let h = synctex_node_box_visible_h(node);
                let v = synctex_node_box_visible_v(node) + synctex_node_box_visible_h(node);
                let x = synctex_node_visible_h(node);
                let y = synctex_node_visible_v(node);
                let width = synctex_node_box_visible_width(node).abs();
                let height = (synctex_node_box_visible_height(node)
                    + synctex_node_box_visible_depth(node))
                .max(1.0);
                let single_pos = PdfPosResp::from((page, h, v, width, height, x, y));
                position_list.push(single_pos);
            }
        }
        synctex_scanner_free(scanner);
        return position_list;
    }
}

pub fn get_src_pos(params: &GetSrcPosParams) -> Vec<SrcPosResp> {
    let proj_dir = get_proj_base_dir(&params.project_id, params.create_time);
    let pdf_file_name = format!("{}{}", get_filename_without_ext(&params.main_file), ".pdf");
    let file_path = join_paths(&[&proj_dir, &pdf_file_name.to_string()]);
    unsafe {
        let c_file_path = CString::new(file_path.clone());
        if let Err(e) = c_file_path {
            error!("parse out path error,{},{}", e, file_path.clone());
            return Vec::new();
        }
        let c_build_path = CString::new(proj_dir.clone());
        if let Err(e) = c_build_path {
            error!("parse build path error,{},{}", e, proj_dir.clone());
            return Vec::new();
        }
        let cstring_file_path = c_file_path.unwrap();
        let cstring_build_path = c_build_path.unwrap();
        let scanner = synctex_scanner_new_with_output_file(
            cstring_file_path.as_ptr(),
            cstring_build_path.as_ptr(),
            1,
        );
        let mut position_list: Vec<SrcPosResp> = Vec::new();
        let node_number = synctex_edit_query(scanner, params.page as c_int, params.h, params.v);
        if node_number > 0 {
            for _i in 0..node_number {
                let node: synctex_node_p = synctex_scanner_next_result(scanner);
                let file = synctex_scanner_get_name(scanner, synctex_node_tag(node));
                let line = synctex_node_line(node);
                let column = synctex_node_column(node);
                let c_str = CStr::from_ptr(file);
                let file_name: String = c_str.to_string_lossy().into_owned();
                let src_relative_path = get_file_relative_path(file_name.clone(), proj_dir.clone());
                let single_pos = SrcPosResp::from((src_relative_path, line, column));
                position_list.push(single_pos);
            }
        }
        synctex_scanner_free(scanner);
        return position_list;
    }
}

fn get_file_relative_path(file_full_path: String, proj_dir: String) -> String {
    let abs_path = Path::new(file_full_path.as_str());
    let root = Path::new(proj_dir.as_str());
    match abs_path.strip_prefix(root) {
        Ok(relative) => {
            let mut relative_path = PathBuf::from(relative);
            let path_string = relative_path.as_mut_os_str().to_string_lossy().to_string();
            let final_path = path_string.replace("./", "");
            return final_path;
        }
        Err(err) => {
            error!("Failed to get relative path: {}", err);
            return "".to_owned();
        }
    }
}
