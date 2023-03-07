use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION};
use rocket::{form::FromForm, post, response::content};
use serde::{Deserialize, Serialize};
use std::error::Error;

use crate::response::{response_error, response_ok};
use rocket::serde::json::Json;

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
}

// 定义响应数据类型
#[derive(Debug, Serialize, Deserialize)]
pub struct Choice {
    pub index: i32,
    pub message: Message,
    pub finish_reason: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: i32,
    pub completion_tokens: i32,
    pub total_tokens: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CompletionResponse {
    pub id: String,
    pub object: String,
    pub created: i64,
    pub choices: Vec<Choice>,
    pub usage: Usage,
}

#[derive(FromForm, Debug, Serialize, Deserialize)]
pub struct ChatRequest<'r> {
    pub content: &'r str,
    pub max_tokens: Option<u64>,
    pub temperature: Option<f64>,
}

pub async fn openai_nlp(
    content: String,
    tokens: Option<u64>,
    temp: Option<f64>,
) -> Result<String, Box<dyn Error>> {
    let url = "https://api.openai.com/v1/chat/completions";
    let max_tokens = tokens.unwrap_or(2048);
    let temperature = temp.unwrap_or(0.2);
    let model = "gpt-3.5-turbo".to_owned();
    let role = "user".to_owned();
    let messages = vec![Message { role, content }];

    let request_data = CompletionRequest {
        max_tokens,
        temperature,
        model,
        messages,
    };

    let api_key = "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";

    let mut headers = HeaderMap::new();
    headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {}", api_key))?,
    );

    let client = reqwest::Client::builder()
        .proxy(reqwest::Proxy::https("http://192.168.2.25:7890")?)
        .build()?;
    let response = client
        .post(url)
        .headers(headers)
        .json(&request_data)
        .send()
        .await?
        // .text()
        .json::<CompletionResponse>()
        .await?;
    let msg = response.choices[0].message.content.trim();
    log::info!(
        "total_tokens = {:#?}, message= {}",
        response.usage.total_tokens,
        msg
    );

    Ok(msg.to_owned())
}

#[post("/openai/chat", format = "json", data = "<req>")]
pub async fn openai_chat(req: Json<ChatRequest<'_>>) -> content::RawJson<String> {
    log::info!("start openai_chat .. ");
    let res =
        crate::openai::openai_nlp(req.content.to_owned(), req.max_tokens, req.temperature).await;

    if let Ok(r) = res {
        response_ok(serde_json::to_value(r).unwrap())
    } else {
        let err = res.err().unwrap().to_string();
        response_error(err)
    }
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
    async fn test_nlp2() {
        crate::config::init_config();
        let prompt = r#"Answer me "what is the relation between FXR and NLRP3" in one sentence after reading the following paragraph. Here is the paragraph: Emerging evidence from animal and human studies has suggested that small microbial metabolites generated in the gut influence host mood and behavior. Our previous study reported that patients with major depressive disorder (MDD) reduced the abundance of genera Blautia and Eubacterium, the microbials critically regulating cholesterol and bile acid metabolism in the gut. In this study, we further demonstrated that the levels of plasma bile acid chenodeoxycholic acid (CDCA) were significantly lower in Chinese MDD patients (142) than in healthy subjects (148). Such low levels of plasma CDCA in MDD patients were rescued in remitters but not in nonremitters following antidepressant treatment. In a parallel animal study, Chronic Social Defeat Stress (CSDS) depressed mice reduced the plasma CDCA and expression level in prefrontal cortex (PFC) of bile acid receptor (FXR) protein, which is a ligand-activated transcription factor and a member of the nuclear receptor superfamily. We found that CDCA treatment restored the level of FXR in the CSDS mice, suggesting the involvement of bile acid receptors in MDD. We observed that CDCA decreased the activity of the NLRP3 inflammasome and caspase-1 and subsequently increased the levels of phosphorylation and expression of PFC glutamate receptors (GluA1) in the PFC. In addition, CDCA showed antidepressant effects in the tests of sucrose preference, tail suspension, and forced swimming in CSDS mouse model of depression. Finally, in agreement with this idea, blocking these receptors by a FXR antagonist GS abolished CDCA-induced antidepressant effect. Moreover, CDCA treatment rescued the increase of IL-1β, IL-6, TNF α and IL-17, which also were blocked by GS. These results suggest that CDCA is a biomarker and target potentially important for the diagnosis and treatment of MDD."#;
        log::info!(
            "result = {:?}",
            openai_nlp(prompt.to_owned(), None, None).await
        );
    }
}
