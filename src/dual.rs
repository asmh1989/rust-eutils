use std::collections::HashMap;
use tokio::sync::RwLock;

use log::info;
use once_cell::sync::Lazy;
use rocket::{get, response::content};
use serde::{Deserialize, Serialize};

use crate::{
    response::response_ok,
    utils::{file_exist, read_target_csv},
};

#[derive(Debug, Deserialize, Serialize)]
struct Motifs {
    #[serde(rename = "SMILES")]
    pub smiles: String,
    #[serde(rename = "mean")]
    pub mean: f64,
    #[serde(rename = "std")]
    pub std: f64,
    #[serde(rename = "title")]
    pub title: String,
    #[serde(rename = "median")]
    pub median: f64,

    #[serde(rename = "ArticleDOI")]
    pub article_doi: Option<String>,
    #[serde(rename = "PatentNumber")]
    pub patent_number: Option<String>,
    pub img1: Option<String>,
    pub img2: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct GenCPDs {
    #[serde(rename = "Left_scaffold")]
    pub left_scaffold: String,
    #[serde(rename = "Right_scaffold")]
    pub right_scaffold: String,
    #[serde(rename = "Left_Frag")]
    pub left_frag: String,
    #[serde(rename = "Right_Frag")]
    pub right_frag: String,
    #[serde(rename = "SMILES")]
    pub smiles: String,
    // #[serde(rename = "Wt")]
    // pub wt: f64,
    // #[serde(rename = "Pass_filter")]
    // pub pass_filter: i32,
    // #[serde(rename = "SA")]
    // pub sa: f64,
    // #[serde(rename = "QED")]
    // pub qed: f64,
    // #[serde(rename = "LogP")]
    // pub logp: f64,
    #[serde(rename = "title")]
    pub title: String,
    #[serde(rename = "Left_mean")]
    pub left_frag_mean: f64,
    #[serde(rename = "Left_std")]
    pub left_frag_std: f64,
    #[serde(rename = "Left_median")]
    pub left_frag_median: f64,
    #[serde(rename = "Right_mean")]
    pub right_frag_mean: f64,
    #[serde(rename = "Right_std")]
    pub right_frag_std: f64,
    #[serde(rename = "Right_median")]
    pub right_frag_median: f64,

    #[serde(rename = "Left_ArticleDOI")]
    pub left_article_doi: String,
    #[serde(rename = "Left_PatentNumber")]
    pub left_patent_number: String,
    #[serde(rename = "Right_ArticleDOI")]
    pub right_article_doi: String,
    #[serde(rename = "Right_PatentNumber")]
    pub right_patent_number: String,

    #[serde(rename = "JAK1ToJAK2_mean", skip_serializing_if = "Option::is_none")]
    pub jak1_to_jak2_mean: Option<f64>,
    #[serde(rename = "JAK1ToJAK2_std", skip_serializing_if = "Option::is_none")]
    pub jak1_to_jak2_std: Option<f64>,
    #[serde(rename = "JAK1ToJAK2_median", skip_serializing_if = "Option::is_none")]
    pub jak1_to_jak2_median: Option<f64>,
}

fn get_dual_list() -> Vec<String> {
    let mut sub = Vec::new();

    if let Ok(dir) = std::fs::read_dir("data/dual") {
        sub = dir
            .filter_map(|f| {
                f.ok().and_then(|e| {
                    if e.path().is_dir() {
                        let ff = e.file_name();
                        let p = ff.to_str().unwrap();
                        if p.contains("-") {
                            Some(p.to_string())
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
            })
            .collect();
    }

    sub
}

#[get("/dual/list")]
pub async fn dual_list() -> content::RawJson<String> {
    info!("dual_list ..");
    response_ok(serde_json::to_value(get_dual_list()).unwrap())
}

fn get_target_info(target: &str, left: bool) -> Vec<Motifs> {
    if target.contains("-") {
        let v: Vec<&str> = target.split("-").collect();
        let t = if left { v[0] } else { v[1] };
        let csv = format!("data/dual/{}/{}/Motifs.csv", target, t);
        if !file_exist(&csv) {
            log::warn!("{} not exist, please check!!", &csv);
        } else {
            let mut vv: Vec<Motifs> = Vec::new();
            let result = read_target_csv(&csv, b',', &mut vv);

            if result.is_err() {
                log::info!("csv error {:?}", result.err());
            }
            vv.iter_mut().for_each(|f| {
                (*f).img1 = Some(format!("/dual/{}/{}/Motifs/{}.png", target, t, &f.title));
                (*f).img2 = Some(format!(
                    "/dual/{}/{}/Motifs/{}_dist.png",
                    target, t, &f.title
                ));
            });

            return vv;
        }
    }

    vec![]
}

// 使用 Lazy 宏创建全局 RwLock<HashMap> 对象
static GLOBAL_MAP: Lazy<RwLock<HashMap<String, Vec<GenCPDs>>>> = Lazy::new(|| {
    let map = HashMap::new();
    RwLock::new(map)
});

// 为 Map 对象生成 insert 方法
async fn insert_to_global_map(key: String, value: Vec<GenCPDs>) {
    let mut map = GLOBAL_MAP.write().await;
    // 写入操作需要获取写锁
    map.insert(key, value);
}

async fn contains_to_global_map(key: String) -> bool {
    let map = GLOBAL_MAP.read().await;
    return map.contains_key(&key);
}

async fn choose_from_global_map(key: String, left: &str, right: &str) -> Option<Vec<GenCPDs>> {
    let map = GLOBAL_MAP.read().await;
    if let Some(v) = map.get(&key) {
        let v2: Vec<GenCPDs> = v
            .iter()
            .filter_map(|f| {
                if f.left_scaffold == left && f.right_scaffold == right {
                    Some(f.clone())
                } else {
                    None
                }
            })
            .collect();
        Some(v2)
    } else {
        None
    }
}

async fn get_gen_cpds(target: &str, left: &str, right: &str) -> Vec<GenCPDs> {
    if target.contains("-") && !contains_to_global_map(target.to_string()).await {
        let csv = format!("data/dual/{}/GenCPDs/GenCPDs.csv", target);
        if !file_exist(&csv) {
            log::warn!("{} not exist, please check!!", &csv);
        } else {
            let mut vv: Vec<GenCPDs> = Vec::new();
            let result = read_target_csv(&csv, b',', &mut vv);

            if result.is_err() {
                log::info!("csv error {:?}", result.err());
            }

            insert_to_global_map(target.to_string(), vv).await;
        }
    }

    if let Some(v) = choose_from_global_map(target.to_string(), left, right).await {
        v
    } else {
        vec![]
    }
}

#[get("/dual/<target>/<left_or_right>")]
pub async fn dual_target_info(target: String, left_or_right: usize) -> content::RawJson<String> {
    info!(
        "dual_target_info .. target = {} left_or_right = {}",
        &target, left_or_right
    );
    response_ok(serde_json::to_value(get_target_info(&target, left_or_right == 0)).unwrap())
}

#[get("/dual/<target>/<left>/<right>")]
pub async fn dual_gen_cpds(
    target: String,
    left: String,
    right: String,
) -> content::RawJson<String> {
    info!(
        "dual_gen_cpds .. target = {} left = {}, right = {}",
        &target, &left, &right
    );
    response_ok(serde_json::to_value(get_gen_cpds(&target, &left, &right).await).unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dual_lsit() {
        crate::config::init_config();

        let r = get_dual_list();

        info!("r = {:?}", &r);
    }

    #[test]
    fn test_get_target_info() {
        crate::config::init_config();
        let r = get_target_info("AMPK-JAK1", true);
        let r2 = get_target_info("AMPK-JAK1", false);

        info!("r = {:?}", r.len());

        info!("r2 = {:?}", &r2);
    }

    #[tokio::test]
    async fn test_get_gen_cpds() {
        crate::config::init_config();
        let r = get_gen_cpds(
            "AMPK-JAK",
            "*c1ccc(-c2ccc3[nH]c(OC4CCCOC4)nc3c2)cc1",
            "*N1CCC(Nc2ncnc3[nH]ccc23)C1",
        )
        .await;

        info!("r = {}", serde_json::to_string_pretty(&r).unwrap());
    }
}
