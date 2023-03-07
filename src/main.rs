#![allow(dead_code)]

use rocket::form::Form;
use rocket::form::FromForm;
use rocket::post;
use std::net::Ipv4Addr;

use rocket::serde::json::Json;

use response::response_ok;
use rocket::{
    fairing::{Fairing, Info, Kind},
    fs::{relative, NamedFile},
    get,
    http::Header,
    info, launch,
    log::LogLevel,
    response::content,
    routes, Request, Response,
};
use utils::{file_exist, get_download_path_by_time, get_pmid_path_by_id};

use crate::openai::ChatRequest;
use crate::response::response_error;

mod config;
mod eutils;
mod model;
mod openai;
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

/// 下载
#[get("/pubmed/save/<term>?<cur_page>&<page_size>&<file_type>")]
async fn query_pubmed_and_save(
    term: String,
    cur_page: Option<usize>,
    page_size: Option<usize>,
    file_type: Option<String>,
) -> content::RawJson<String> {
    info!(
        "pubmed query = {:?}, file_type = {:?}",
        term.as_str(),
        &file_type
    );

    let res = crate::eutils::esearch3("pubmed", &term, cur_page, page_size).await;
    if let Ok(r) = res {
        match model::PaperCsvResult::save_list_csv(&r) {
            Ok(rr) => response_ok(serde_json::to_value(rr).unwrap()),
            Err(err) => response_error(err.to_string()),
        }
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

#[get("/<file_type>/<id>")]
async fn download(file_type: String, id: i64) -> Option<NamedFile> {
    let path = get_download_path_by_time(&file_type, id);
    if file_exist(&path) {
        NamedFile::open(&path).await.ok()
    } else {
        None
    }
}

#[post("/openai/chat", format = "json", data = "<req>")]
async fn openai_chat(req: Json<ChatRequest<'_>>) -> content::RawJson<String> {
    log::info!(" start openai_chat .. ");
    let res =
        crate::openai::openai_nlp(req.content.to_owned(), req.max_tokens, req.temperature).await;

    if let Ok(r) = res {
        response_ok(serde_json::to_value(r).unwrap())
    } else {
        let err = res.err().unwrap().to_string();
        response_error(err)
    }
}

#[post("/openai/chat2", data = "<req>")]
async fn openai_chat_form(req: Form<ChatRequest<'_>>) -> content::RawJson<String> {
    log::info!(" start openai_chat .. {:?}", &req);
    let res =
        crate::openai::openai_nlp(req.content.to_owned(), req.max_tokens, req.temperature).await;

    if let Ok(r) = res {
        response_ok(serde_json::to_value(r).unwrap())
    } else {
        let err = res.err().unwrap().to_string();
        response_error(err)
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
        .attach(Cors) // cors
        .mount(
            "/api",
            routes![
                query_pubmed,
                get_pubmed_by_id,
                query_pubmed_total,
                query_pubmed_and_save,
                openai_chat_form,
                openai_chat
            ],
        )
        .mount(
            "/",
            rocket::fs::FileServer::from(relative!("../vue-eutils/dist")),
        )
        .mount("/", routes![get_pubmed])
        .mount("/download", routes![download])
}

pub struct Cors;

#[rocket::async_trait]
impl Fairing for Cors {
    fn info(&self) -> Info {
        Info {
            name: "Cross-Origin-Resource-Sharing Fairing",
            kind: Kind::Response,
        }
    }

    async fn on_response<'r>(&self, _request: &'r Request<'_>, response: &mut Response<'r>) {
        response.set_header(Header::new("Access-Control-Allow-Origin", "*"));
        response.set_header(Header::new(
            "Access-Control-Allow-Methods",
            "POST, PATCH, PUT, DELETE, HEAD, OPTIONS, GET",
        ));
        response.set_header(Header::new("Access-Control-Allow-Headers", "*"));
        response.set_header(Header::new("Access-Control-Allow-Credentials", "true"));
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_shell() {
        crate::config::init_config();
    }
}
