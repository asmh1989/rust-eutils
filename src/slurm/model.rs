use std::path::Path;

use bson::DateTime;
use mongodb::bson::{self, Document};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct Job {
    #[serde(rename = "User")]
    pub user: String,

    #[serde(rename = "JobID")]
    pub job_id: u32,

    #[serde(rename = "JobName")]
    pub job_name: String,

    #[serde(rename = "Start")]
    pub start: String,

    #[serde(rename = "End")]
    pub end: String,

    #[serde(rename = "ElapsedRaw")]
    pub elapsed_raw: u32,

    #[serde(rename = "WorkDir")]
    pub work_dir: String,

    #[serde(rename = "State")]
    pub state: String,

    #[serde(rename = "NodeList")]
    pub node_list: String,

    #[serde(rename = "AllocTRES")]
    pub alloc_tres: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct JobInDb {
    pub user: String,
    pub job_id: u32,
    pub job_name: String,
    pub start: Option<DateTime>,
    pub end: Option<DateTime>,
    pub elapsed_raw: u32,
    pub work_dir: String,
    pub state: String,
    pub node_list: String,
    pub cpu: u32,
    pub gpu: u32,
    pub mem: String,
    pub stdout: Option<String>,
    pub cloud: String,
    pub is_send: Option<u32>,
}
pub const ROOT_PATH: &'static str = "/mnt/share/cloud_sbatch";

fn get_info_path(path: &str) -> (&str, &str) {
    let path_obj = Path::new(path);

    let root = match path_obj.parent() {
        Some(parent) => match parent.parent() {
            Some(parent2) => parent2.to_str().unwrap(),
            None => "",
        },
        None => "",
    };

    let md = match path_obj.parent() {
        Some(parent) => match parent.file_name() {
            Some(parent2) => parent2.to_str().unwrap(),
            None => "",
        },
        None => "",
    };

    (root, md)
}

impl JobInDb {
    pub fn document(&self) -> Result<Document, String> {
        match bson::to_bson(&self) {
            Ok(d) => {
                let mut doc = d.as_document().unwrap().clone();
                doc.remove("is_send");
                return Ok(doc);
            }
            Err(e) => {
                log::info!("to_bson err {}", e);
                return Err(format!("to_bson error : {}", e));
            }
        };
    }
    pub fn work_dir_root(&self) -> String {
        let (x, _) = get_info_path(&self.work_dir);
        x.to_string()
    }

    pub fn md_name(&self) -> String {
        let (_, x) = get_info_path(&self.work_dir);
        x.to_string()
    }

    pub fn tgz_name(&self) -> String {
        let md = self.md_name();
        format!("{}-out-xvg.tgz", md)
    }

    pub fn local_dir(&self) -> String {
        let vv = self.work_dir.split(&self.user).collect::<Vec<&str>>();
        let local_dir = if vv.len() > 1 { vv[1] } else { "/tgz" };
        let dir = format!("{}{}", ROOT_PATH, local_dir);
        let (d, _) = get_info_path(&dir);
        if d.is_empty() {
            format!("{}{}", ROOT_PATH, local_dir)
        } else {
            d.to_string()
        }
    }
    pub fn local_tgz(&self) -> String {
        format!("{}/{}", self.local_dir(), self.tgz_name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dir() {
        let (a, b) = get_info_path(
            "/mnt/share/cloud_sbatch/yrl/abfep/jak1/compare-new-restr/PRK-560_md/fep/",
        );
        println!("a = {}, b = {}", a, b);
    }
}
