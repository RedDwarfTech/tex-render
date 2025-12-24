use crate::model::cv::{
     lang::cv_lang_resp::CvLangResp, project::cv_project_resp::CvProjectResp,
    skill::cv_skill_resp::CvSkillResp, work::cv_work_resp::CvWorkResp,
};
use crate::model::project::compile_app_params::CompileAppParams;
use log::error;
use rust_wheel::common::util::rd_file_util::{create_folder_not_exists, get_filename_without_ext, join_paths};
use std::fs;

pub fn gen_work_items(content: String) -> String {
    if content.is_empty() {
        return content;
    }
    let content_array = content.split("* ");
    let mut content = "".to_string();
    content.push_str(&format!("{}", "\\begin{itemize}\n"));
    content_array.for_each(|item| {
        if !item.is_empty() {
            content.push_str(&format!("{}{}", "\\item ", item));
        }
    });
    content.push_str(&format!("{}", "\n\\end{itemize}\n"));
    return content;
}

pub fn get_work_str(works: &Option<Vec<CvWorkResp>>) -> String {
    match works {
        Some(edu) => {
            let mut s = String::new();
            for i in edu {
                let parts: Vec<&str> = i.work_start.as_ref().unwrap().split("-").collect();
                let work_start = format!("{}.{}", parts[0], parts[1]);
                let parts_graduation: Vec<&str> = i.work_end.as_ref().unwrap().split("-").collect();
                let work_end: String = format!("{}.{}", parts_graduation[0], parts_graduation[1]);
                let work_item_content =
                    gen_work_items(i.duty.as_deref().unwrap_or_default().to_string());
                s += &format!(
                    "{}{}{}{}{}{}{}{}{}{}{}{}{}",
                    "\\cventry{",
                    work_start,
                    "--",
                    work_end,
                    "}{",
                    i.job.as_ref().unwrap().to_string(),
                    "}{",
                    i.company.to_owned(),
                    "}{",
                    i.city.as_ref().unwrap().to_owned(),
                    "}{}{\n",
                    work_item_content,
                    "}\n\n"
                )
                .to_string();
            }
            return s;
        }
        None => return "".to_owned(),
    }
}

pub fn get_skill_str(works: &Option<Vec<CvSkillResp>>) -> String {
    match works {
        Some(edu) => {
            let mut s = String::new();
            for i in edu {
                s += &format!(
                    "{}{}{}{}{}",
                    "\\cvitem{",
                    i.name.clone(),
                    "}{\\small ",
                    i.memo.as_ref().unwrap().to_string(),
                    "}\n"
                )
                .to_string();
            }
            return s;
        }
        None => return "".to_owned(),
    }
}

pub fn get_project_str(works: &Option<Vec<CvProjectResp>>) -> String {
    match works {
        Some(edu) => {
            let mut s = String::new();
            for i in edu {
                let parts: Vec<&str> = i.work_start.as_ref().unwrap().split("-").collect();
                let work_start = format!("{}.{}", parts[0], parts[1]);
                let parts_graduation: Vec<&str> = i.work_end.as_ref().unwrap().split("-").collect();
                let work_end = format!("{}.{}", parts_graduation[0], parts_graduation[1]);
                let work_item_content =
                    gen_work_items(i.duty.as_deref().unwrap_or_default().to_string());
                s += &format!(
                    "{}{}{}{}{}{}{}{}{}{}{}{}{}",
                    "\\cventry{",
                    work_start,
                    "--",
                    work_end,
                    "}{",
                    i.name.clone(),
                    "}{",
                    i.company.as_deref().unwrap_or_default().to_owned(),
                    "}{",
                    i.city.as_deref().unwrap_or_default(),
                    "}{}{\n",
                    work_item_content,
                    "}\n\n"
                )
                .to_string();
            }
            return s;
        }
        None => return "".to_owned(),
    }
}

/**
 * Copy the compiled pdf to output directory.
 * When the tex project was compiling, loading the compile folder's pdf file will fail,
 * so we copy the file to a separate output folder where the compiled pdf is stored.
 */
pub fn copy_pdf_to_output_dir(params: &CompileAppParams, compile_dir: &str) -> String{
    let pdf_output_folder = join_paths(&[compile_dir.to_owned(), "app-compile-output".to_owned()]);
    create_folder_not_exists(&pdf_output_folder);

    let fpath = params.file_path.clone();
    let name_without_ext = get_filename_without_ext(&fpath);
    let pdf_compile_path = join_paths(&[
        compile_dir.to_owned(),
        format!("{}{}", name_without_ext.to_owned(), ".pdf".to_owned()),
    ]);
    let pdf_output_path = join_paths(&[
        pdf_output_folder,
        format!("{}{}", name_without_ext.to_owned(), ".pdf".to_owned()),
    ]);

    let cp_result = fs::copy(&pdf_compile_path, &pdf_output_path);
    if let Err(err) = cp_result {
        error!(
            "copy pdf to output folder failed, {}, output: {}",
            err, pdf_output_path
        );
    }
    return pdf_compile_path;
}
