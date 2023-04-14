use async_recursion::async_recursion;
use csv::{QuoteStyle, WriterBuilder};
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION};
use rocket::{form::FromForm, post, response::content};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::{error::Error, time::Duration};
use tokio::time::sleep;

use crate::model::PaperCsvResult;
use crate::response::{response_error, response_ok};
use rocket::serde::json::Json;

use rocket::form::Form;
use rocket::fs::{NamedFile, TempFile};

#[derive(Debug, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

// 定义请求数据类型
#[derive(Debug, Serialize, Deserialize)]
pub struct CompletionRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub max_tokens: u64,
    pub temperature: f64,
    pub top_p: f64,
    pub presence_penalty: f64,
}

// 定义响应数据类型
#[derive(Debug, Serialize, Deserialize)]
pub struct Choice {
    pub index: i32,
    pub message: Message,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: i32,
    pub completion_tokens: i32,
    pub total_tokens: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CompletionResponse {
    pub id: Option<String>,
    pub object: String,
    pub created: i64,
    pub choices: Vec<Choice>,
    pub usage: Usage,
}

#[derive(FromForm, Debug, Serialize, Deserialize)]
pub struct ChatRequest {
    pub content: String,
    pub max_tokens: Option<u64>,
    pub temperature: Option<f64>,
}

const DEALY_TIME: u64 = 8;
const USE_CHATGPT_API: bool = true;
#[async_recursion]
pub async fn openai_nlp(
    content: String,
    tokens: Option<u64>,
    temp: Option<f64>,
) -> Result<String, Box<dyn Error + Send + Sync>> {
    let text = if USE_CHATGPT_API {
        let url = "http://192.168.2.212:3002/api/sendMessage";
        let mut headers = HeaderMap::new();
        headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
        let client = reqwest::Client::builder().build()?;

        let request_data = serde_json::json!({
            "content": content.clone(),
        });

        let res = client
            .post(url)
            .headers(headers)
            .json(&request_data)
            .send()
            .await?;
        res.text().await?
    } else {
        let max_tokens = tokens.unwrap_or(2048);
        let temperature = temp.unwrap_or(0.8);
        let model = "gpt-3.5-turbo".to_owned();
        let role = "user".to_owned();
        let messages = vec![Message {
            role,
            content: content.clone(),
        }];

        let presence_penalty = 1.0;
        let top_p = 1.0;

        let request_data = CompletionRequest {
            max_tokens,
            temperature,
            model,
            messages,
            top_p,
            presence_penalty,
        };
        let url = "https://api.openai.com/v1/chat/completions";

        let api_key = "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";

        let mut headers = HeaderMap::new();
        headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", api_key))?,
        );

        let short_string: String = content
            .split_whitespace()
            .take(16)
            .collect::<Vec<_>>()
            .join(" ");
        log::info!("start openai_content = {}...", short_string);
        let client = reqwest::Client::builder()
            .proxy(reqwest::Proxy::https("http://192.168.2.25:7890")?)
            .build()?;

        let res = client
            .post(url)
            .headers(headers)
            .json(&request_data)
            .send()
            .await?;
        res.text().await?
    };

    match serde_json::from_str::<CompletionResponse>(&text) {
        Ok(response) => {
            let msg = response.choices[0].message.content.trim();
            log::info!(
                "total_tokens = {:#?}, message = {}",
                response.usage.total_tokens,
                msg
            );

            Ok(msg.to_owned())
        }
        Err(err) => {
            log::info!("res = {}, err = {:?}", &text, err);
            sleep(Duration::from_secs(DEALY_TIME)).await;

            openai_nlp(content, tokens, temp).await
        }
    }
}

#[derive(FromForm)]
pub struct Upload<'r> {
    pub question: &'r str,
    pub file: TempFile<'r>,
}

#[post("/openai/summary", data = "<req>")]
pub async fn openai_chat_summary_file(req: Form<Upload<'_>>) -> Option<NamedFile> {
    let res = req.file.path();
    if res.is_none() {
        log::info!("summary temp file path = {:?}, failed", &res);
        None
    } else {
        let p = res.unwrap();

        log::info!("summary path={:?}, question={}", p, req.question);
        match chat_abstract_summary(p, req.question).await {
            Ok(pp) => NamedFile::open(&pp).await.ok(),
            Err(err) => {
                log::info!("summary error = {:?}", err);
                None
            }
        }
    }
}

#[post("/openai/chat", format = "json", data = "<req>")]
pub async fn openai_chat(req: Json<ChatRequest>) -> content::RawJson<String> {
    let res =
        crate::openai::openai_nlp(req.content.to_owned(), req.max_tokens, req.temperature).await;

    if let Ok(r) = res {
        response_ok(serde_json::to_value(r).unwrap())
    } else {
        let err = res.err().unwrap().to_string();
        response_error(err)
    }
}

fn save_to_file<T: Serialize>(
    name: &str,
    v: &Vec<T>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut writer = WriterBuilder::new()
        .quote_style(QuoteStyle::Necessary)
        .from_path(name)?;

    for person in v {
        writer.serialize(person)?;
    }

    writer.flush()?;

    Ok(())
}

async fn chat_abstract_summary<P: AsRef<Path>>(
    path: P,
    question: &str,
) -> Result<String, Box<dyn Error + Send + Sync>> {
    let mut v: Vec<PaperCsvResult> = Vec::new();

    crate::utils::read_target_csv(path, b',', &mut v)?;

    let mut rr = Vec::with_capacity(v.len());

    for f in v {
        let content = format!(
            "Answer me {} in one sentence after reading the following paragraph: {}",
            question, &f.r#abstract
        );
        let csv = f;

        let summary = loop {
            let res = openai_nlp(content.clone(), None, None).await;
            match res {
                Ok(s) => {
                    break s;
                }
                Err(err) => {
                    log::info!("openai_nlp error = {:?}, will retry", err);
                }
            }

            sleep(Duration::from_secs(DEALY_TIME)).await;
        };

        rr.push(csv.to_summary(summary));
    }

    let (file_name, _) = crate::utils::get_download_path("csv")?;
    save_to_file(&file_name, &rr)?;

    Ok(file_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_nlp1() {
        crate::config::init_config();
        let prompt = "hello".to_owned();
        log::info!("result = {:?}", openai_nlp(prompt, None, None).await);
    }

    #[tokio::test]
    async fn test_summary() {
        crate::config::init_config();
        log::info!(
            "result = {:?}",
            chat_abstract_summary(
                "data/paper.csv",
                "what is the relation between FXR and NLRP3"
            )
            .await
        );
    }

    #[tokio::test]
    async fn test_nlp2() {
        crate::config::init_config();
        let prompt = r#"Answer me "what is the relation between FXR and NLRP3" in one sentence after reading the following paragraph. Here is the paragraph: Emerging evidence from animal and human studies has suggested that small microbial metabolites generated in the gut influence host mood and behavior. Our previous study reported that patients with major depressive disorder (MDD) reduced the abundance of genera Blautia and Eubacterium, the microbials critically regulating cholesterol and bile acid metabolism in the gut. In this study, we further demonstrated that the levels of plasma bile acid chenodeoxycholic acid (CDCA) were significantly lower in Chinese MDD patients (142) than in healthy subjects (148). Such low levels of plasma CDCA in MDD patients were rescued in remitters but not in nonremitters following antidepressant treatment. In a parallel animal study, Chronic Social Defeat Stress (CSDS) depressed mice reduced the plasma CDCA and expression level in prefrontal cortex (PFC) of bile acid receptor (FXR) protein, which is a ligand-activated transcription factor and a member of the nuclear receptor superfamily. We found that CDCA treatment restored the level of FXR in the CSDS mice, suggesting the involvement of bile acid receptors in MDD. We observed that CDCA decreased the activity of the NLRP3 inflammasome and caspase-1 and subsequently increased the levels of phosphorylation and expression of PFC glutamate receptors (GluA1) in the PFC. In addition, CDCA showed antidepressant effects in the tests of sucrose preference, tail suspension, and forced swimming in CSDS mouse model of depression. Finally, in agreement with this idea, blocking these receptors by a FXR antagonist GS abolished CDCA-induced antidepressant effect. Moreover, CDCA treatment rescued the increase of IL-1β, IL-6, TNF α and IL-17, which also were blocked by GS. These results suggest that CDCA is a biomarker and target potentially important for the diagnosis and treatment of MDD."#;
        log::info!(
            "result = {:?}",
            openai_nlp(prompt.to_owned(), None, None).await
        );
    }
}
