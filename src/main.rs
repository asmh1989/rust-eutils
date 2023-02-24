#![allow(dead_code)]

use std::net::Ipv4Addr;

use response::response_ok;
use rocket::{fs::NamedFile, get, info, launch, log::LogLevel, response::content, routes};
use utils::{file_exist, get_pmid_path_by_id};

use crate::response::response_error;

mod config;
mod eutils;
mod model;
mod response;
mod utils;

#[get("/pubmed/<term>?<cur_page>&<page_size>")]
async fn query_pubmed(
    term: String,
    cur_page: Option<usize>,
    page_size: Option<usize>,
) -> content::RawJson<String> {
    info!("pubmed query = {:?}", term.as_str());

    let res = crate::eutils::esearch2("pubmed", &term, cur_page, page_size).await;
    if let Ok(r) = res {
        response_ok(serde_json::to_value(r).unwrap())
    } else {
        let err = res.err().unwrap().to_string();
        response_error(err)
    }
    // response_error("not found".to_string())
}

#[get("/pubmed/total/<term>")]
async fn query_pubmed_total(term: String) -> content::RawJson<String> {
    info!("pubmed query = {:?}", term.as_str());

    let res = crate::eutils::esearch("pubmed", &term).await;
    if let Ok(r) = res {
        response_ok(serde_json::to_value(r).unwrap())
    } else {
        let err = res.err().unwrap().to_string();
        response_error(err)
    }
    // response_error("not found".to_string())
}

#[get("/pubmed/pmid/<pmid>")]
async fn get_pubmed_by_id(pmid: String) -> content::RawJson<String> {
    let res = crate::eutils::efetch("pubmed", &vec![pmid]).await;
    if let Ok(r) = res {
        if r.is_empty() {
            response_error("not found".to_string())
        } else {
            response_ok(serde_json::to_value(r.first().unwrap()).unwrap())
        }
    } else {
        let err = res.err().unwrap().to_string();
        response_error(err)
    }
}

#[get("/pubmed/<pmid>")]
async fn get_pubmed(pmid: String) -> Option<NamedFile> {
    let res = pmid.parse::<usize>();
    if let Ok(id) = res {
        let path = get_pmid_path_by_id(id);
        if file_exist(&path) {
            NamedFile::open(&path).await.ok()
        } else {
            let result = crate::eutils::efetch("pubmed", &vec![pmid]).await;
            if result.is_ok() {
                NamedFile::open(&path).await.ok()
            } else {
                log::info!("downloaded error... {:?}", result);
                None
            }
        }
    } else {
        None
    }
}

#[launch]
async fn rocket() -> _ {
    crate::config::init_config();

    let mut cfg = rocket::config::Config::default();
    cfg.address = Ipv4Addr::new(0, 0, 0, 0).into();
    cfg.log_level = LogLevel::Normal;
    cfg.port = 4321;

    rocket::custom(cfg)
        .mount(
            "/api",
            routes![query_pubmed, get_pubmed_by_id, query_pubmed_total],
        )
        .mount("/", routes![get_pubmed])
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_shell() {
        crate::config::init_config();
    }
}
