use serde::{Deserialize, Serialize};

use crate::utils::{file_exist, read_target_csv};

#[derive(Debug, Deserialize, Serialize)]
pub struct Core {
    #[serde(rename = "SMILES")]
    pub smiles: String,
    #[serde(rename = "Title")]
    pub title: String,
    #[serde(rename = "Score")]
    pub score: f64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Linker {
    #[serde(rename(deserialize = "Linker", serialize = "SMILES"))]
    pub smiles: String,
    #[serde(rename(deserialize = "Link", serialize = "Title"))]
    pub title: String,
    #[serde(rename = "Score")]
    pub score: f64,
}

pub fn get_target_info(target: &str, left: bool) -> Vec<Core> {
    if target.contains("-") {
        let v: Vec<&str> = target.split("-").collect();
        let t = if left { v[0] } else { v[1] };
        let csv = format!("data/dual/{}/{}.csv", target, t);
        if !file_exist(&csv) {
            log::warn!("{} not exist, please check!!", &csv);
        } else {
            let mut vv: Vec<Core> = Vec::new();
            let result = read_target_csv(&csv, b',', &mut vv);

            if result.is_err() {
                log::info!("csv error {:?}", result.err());
            }
            return vv;
        }
    }
    vec![]
}

pub fn get_link(target: &str, left: &str, right: &str) -> Vec<Linker> {
    if target.contains("-") {
        let csv = format!("data/dual/{}/LINK/{}_{}.csv", target, left, right);
        if !file_exist(&csv) {
            log::warn!("{} not exist, please check!!", &csv);
        } else {
            let mut vv: Vec<Linker> = Vec::new();
            let result = read_target_csv(&csv, b',', &mut vv);

            if result.is_err() {
                log::info!("csv error {:?}", result.err());
            }
            return vv;
        }
    }
    vec![]
}

pub fn get_pair(target: &str, link: &str) -> Vec<Core> {
    if target.contains("-") {
        let csv = format!("data/dual/{}/PAIR/{}", target, link);
        if !file_exist(&csv) {
            log::warn!("{} not exist, please check!!", &csv);
        } else {
            let mut vv: Vec<Core> = Vec::new();
            let result = read_target_csv(&csv, b',', &mut vv);

            if result.is_err() {
                log::info!("csv error {:?}", result.err());
            }
            return vv;
        }
    }
    vec![]
}

#[cfg(test)]
mod tests {
    use regex::Regex;

    use crate::utils::{read_target_csv, save_to_file};

    #[test]
    fn test_get_link() {
        crate::config::init_config();

        get_link("AMPK-JAK-LINK", "R0_14", "R0_2");
    }

    #[derive(Debug, Deserialize, Serialize)]
    pub struct Compound {
        #[serde(rename(deserialize = "Core", serialize = "smiles"))]
        pub core: String,

        #[serde(rename(deserialize = "avg_Score", serialize = "score"))]
        pub avg_score: f64,
        #[serde(rename(deserialize = "Link", serialize = "title"))]
        pub link: String,
    }

    use super::*;

    fn csv_2(file: &str, file2: &str) {
        let re = Regex::new(r#"^=HYPERLINK\([^/]+/(?P<filename>[^/]+)\.csv,[^)]+\)$"#).unwrap();
        let mut v: Vec<Compound> = Vec::new();
        let _ = read_target_csv(file, b',', &mut v);
        v.iter_mut().for_each(|f| {
            let input = f.link.replace("\"", "");
            if let Some(captures) = re.captures(&input) {
                if let Some(filename) = captures.name("filename") {
                    println!("{}", filename.as_str());
                    (*f).link = filename.as_str().to_string();
                }
            }
        });

        let _ = save_to_file(file2, &v);
    }

    #[test]
    fn test_data() {
        crate::config::init_config();

        // let file = "data/fuse/AIXB_3_Fuse/sep_by_core1.csv";
        csv_2(
            "data/fuse/AIXB_3_Fuse/sep_by_core1.csv",
            "data/fuse_ampk.csv",
        );
        csv_2(
            "data/fuse/AIXB_3_Fuse/sep_by_core2.csv",
            "data/fuse_jak.csv",
        );
    }
}
