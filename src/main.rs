#![allow(dead_code)]

use std::net::Ipv4Addr;

use response::response_ok;
use rocket::{fs::NamedFile, get, info, launch, log::LogLevel, response::content, routes};
use utils::{file_exist, get_pmid_path_by_id};

mod config;
mod eutils;
mod model;
mod response;
mod utils;

#[get("/pubmed/<term>")]
async fn query_pubmed(term: String) -> content::RawJson<String> {
    info!("pubmed query = {:?}", term.as_str());

    response_ok(serde_json::to_value(term).unwrap())
    // response_error("not found".to_string())
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
        .mount("/api", routes![query_pubmed])
        .mount("/", routes![get_pubmed])
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_shell() {
        crate::config::init_config();
    }
}
