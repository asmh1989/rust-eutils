use rocket::response::content;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
#[derive(Debug, Serialize, Deserialize)]
enum MyHttpReponse {
    #[serde(rename = "ok")]
    Ok(Value),
    #[serde(rename = "error")]
    Error(Value),
}

pub fn response_ok(value: Value) -> content::RawJson<String> {
    content::RawJson(serde_json::to_string(&MyHttpReponse::Ok(value)).unwrap())
}

pub fn response_error(msg: String) -> content::RawJson<String> {
    content::RawJson(serde_json::to_string(&MyHttpReponse::Error(json!({ "msg": msg }))).unwrap())
}
