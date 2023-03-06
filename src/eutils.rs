use std::sync::{Arc, Mutex};

use crate::{
    model::{PaperCsvResult, PubmedArticleSet},
    utils::{file_exist, get_pmid_path_by_id, read_target_csv},
};
use crossbeam_deque::Worker;
use once_cell::sync::OnceCell;
use regex::Regex;
use serde::{Deserialize, Serialize};

static INSTANCE: OnceCell<Arc<Mutex<Worker<i32>>>> = OnceCell::new();
// static CPU_LOCK: OnceCell<Arc<Mutex<usize>>> = OnceCell::new();

pub fn init_work(work: Worker<i32>) {
    INSTANCE
        .set(Arc::new(Mutex::new(work)))
        .expect("work init error");
}

#[derive(Debug, Serialize, Deserialize)]
struct SearchResult {
    esearchresult: ESearchResult,
}

#[derive(Debug, Serialize, Deserialize)]
struct ESearchResult {
    count: String,
    retmax: String,
    retstart: String,
    idlist: Vec<String>,
    // translationset: Vec<String>,
    querytranslation: String,
}

static ESEARCH: &str = "https://eutils.ncbi.nlm.nih.gov/entrez/eutils/esearch.fcgi?";

static EFETCH: &str = "https://eutils.ncbi.nlm.nih.gov/entrez/eutils/efetch.fcgi?";

static PAGE_SIZE: usize = 20;

async fn lock() -> i32 {
    loop {
        let s = {
            let w = INSTANCE.get().expect("work need init first");
            w.lock().unwrap().stealer()
        };

        if let crossbeam_deque::Steal::Success(id) = s.steal() {
            return id;
        } else {
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }
    }
}

fn unlock(id: i32) {
    let w = INSTANCE.get().expect("work need init first");
    w.lock().unwrap().push(id);
}

pub async fn esearch2(
    db: &str,
    query: &str,
    page: Option<usize>,
    page_size: Option<usize>,
) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
    let retstart = page.unwrap_or(0);
    let page_size = page_size.unwrap_or(10);

    let mut ids: Vec<String> = Vec::new();
    let resp = fetch_ids(db, query, retstart, page_size).await?;

    ids.extend(resp.esearchresult.idlist.iter().cloned());

    log::info!("ids len={},  data = {:?}", ids.len(), ids);

    let res = efetch(db, &ids).await?;

    Ok(serde_json::json!({
        "count": resp.esearchresult.count.parse::<i32>().unwrap_or(-1),
        "data": res,
        "cur_page": retstart,
        "page_size": page_size,
        "query_text": resp.esearchresult.querytranslation
    }))
}

async fn fetch_ids(
    db: &str,
    query: &str,
    retstart: usize,
    page_size: usize,
) -> Result<SearchResult, Box<dyn std::error::Error + Send + Sync>> {
    let start = if retstart == 0 {
        retstart
    } else {
        retstart * page_size
    };

    let url = format!(
            "{}db={}&term={}&retmode=json&api_key=f6bc4f0e30a718d326ef842054d988ecdd08&retstart={}&retmax={}",
            ESEARCH,
            &db,
            &url_encode(&query),
            start,
            page_size
        );

    let id = lock().await;
    log::info!("request_task_id = {},  esearch, url = {}", id, &url);
    let resp = reqwest::get(&url).await?.json::<SearchResult>().await?;
    unlock(id);

    Ok(resp)
}

pub async fn esearch3(
    db: &str,
    query: &str,
    page: Option<usize>,
    page_size: Option<usize>,
) -> Result<Vec<PaperCsvResult>, Box<dyn std::error::Error + Send + Sync>> {
    let retstart = page.unwrap_or(0);
    let page_size = page_size.unwrap_or(10);

    let mut ids: Vec<String> = Vec::new();
    let resp = fetch_ids(db, query, retstart, page_size).await?;
    ids.extend(resp.esearchresult.idlist.iter().cloned());

    log::info!("ids len={},  data = {:?}", ids.len(), ids);

    let res = efetch(db, &ids).await?;

    Ok(res)
}

pub async fn esearch(
    db: &str,
    query: &str,
) -> Result<Vec<PaperCsvResult>, Box<dyn std::error::Error + Send + Sync>> {
    let mut retstart = 0;
    let page_size = PAGE_SIZE * 2;

    let mut ids: Vec<String> = Vec::new();

    loop {
        let resp = fetch_ids(db, query, retstart, page_size).await?;
        ids.extend(resp.esearchresult.idlist.iter().cloned());
        // log::info!("{:#?}", resp);

        let count = resp.esearchresult.count.parse::<usize>().unwrap();
        let downloaded = resp.esearchresult.idlist.len() + retstart;

        if downloaded >= count || resp.esearchresult.idlist.is_empty() {
            break;
        }

        // log::info!("{:#?}", resp);
        // tokio::time::sleep(Duration::from_millis(500)).await;

        retstart += page_size;
    }

    log::info!("ids len={},  data = {:?}", ids.len(), ids);

    let res = efetch(db, &ids).await?;

    Ok(res)
}

pub async fn efetch(
    db: &str,
    ids: &Vec<String>,
) -> Result<Vec<PaperCsvResult>, Box<dyn std::error::Error + Send + Sync>> {
    let mut v: Vec<PaperCsvResult> = Vec::new();
    for id in ids {
        let pmid = id.parse::<usize>()?;
        let path = get_pmid_path_by_id(pmid);
        if !file_exist(&path) {
            let url = format!(
            "{}db={}&api_key=f6bc4f0e30a718d326ef842054d988ecdd08&retmode=text&rettype=xml&id={}",
            EFETCH, &db, id
        );
            log::info!("start download {}", pmid);

            let id = lock().await;
            log::info!("request_task_id = {},  efetch, url = {}", id, &url);
            let resp = reqwest::get(&url).await?.text().await?;
            let p = parse_xml(&resp)?;
            v.extend(p);
            unlock(id);
            log::info!("downloaded {} end", pmid);
        } else {
            // log::info!("pmid = {} already downloaded", pmid);

            let result = read_target_csv(&path, &mut v);
            if result.is_err() {
                log::warn!("path = {},  csv parse error = {:?}", &path, result);
                let _ = std::fs::remove_file(&path);
            }
        }
    }

    log::info!("PaperCsvResult len = {:?}", v.len());

    Ok(v)
}

fn remove_str(input: &str, key: &str) -> String {
    let re = Regex::new(&format!("<{}.*?>([\\s\\S]*?)</{}>", key, key)).unwrap();
    let output = re.replace_all(input, |caps: &regex::Captures| {
        let text = caps.get(1).unwrap().as_str();
        // log::info!("remove_str({}) = {}", key, text);

        let re_tags = Regex::new(r#"<[^>]+>"#).unwrap();
        let text = re_tags.replace_all(text, "");
        format!("<{}>{}</{}>", key, text, key)
    });

    output.to_string()
}

fn parse_xml(xml: &str) -> Result<Vec<PaperCsvResult>, Box<dyn std::error::Error + Send + Sync>> {
    let mut text = remove_str(xml, "AbstractText");
    text = remove_str(&text, "ArticleTitle");

    // log::info!(
    //     "xml struct = {}",
    //     serde_json::to_string_pretty(&text).unwrap()
    // );

    let p: PubmedArticleSet = serde_xml_rs::from_str(&text)?;

    let res = p
        .pubmed_article
        .unwrap_or(vec![])
        .iter()
        .map(|f| {
            let mut paper = PaperCsvResult::default();
            paper.pmid = f.medline_citation.pmid.clone();
            paper.title = f.medline_citation.article.article_title.clone();
            paper.pubdate_year = f
                .medline_citation
                .article
                .journal
                .journal_issue
                .pub_date
                .year
                .clone();
            paper.pubdate_month = f
                .medline_citation
                .article
                .journal
                .journal_issue
                .pub_date
                .month
                .clone()
                .unwrap_or("".to_string());
            paper.journal_title = f.medline_citation.article.journal.title.clone();
            paper.journal_abbr = f.medline_citation.article.journal.iso_abbreviation.clone();
            if f.medline_citation.article.r#abstract.is_some() {
                let abs = f
                    .medline_citation
                    .article
                    .r#abstract
                    .as_ref()
                    .unwrap()
                    .abstract_text
                    .iter()
                    .map(|v| v.value.clone())
                    .collect::<Vec<String>>()
                    .join(" ")
                    .replace("\n", "");

                paper.r#abstract = abs;
            }
            let authors = &f.medline_citation.article.author_list.authors;
            paper.author_first = format!(
                "{} {}",
                authors
                    .first()
                    .unwrap()
                    .fore_name
                    .clone()
                    .unwrap_or("".to_string()),
                authors
                    .first()
                    .unwrap()
                    .last_name
                    .clone()
                    .unwrap_or("".to_string())
            );

            if authors.len() > 1 {
                paper.author_last = format!(
                    "{} {}",
                    authors
                        .last()
                        .unwrap()
                        .fore_name
                        .clone()
                        .unwrap_or("".to_string()),
                    authors
                        .last()
                        .unwrap()
                        .last_name
                        .clone()
                        .unwrap_or("".to_string())
                );
            }

            paper.publication_type = f
                .medline_citation
                .article
                .publication_type_list
                .publication_types
                .iter()
                .map(|s| s.value.clone())
                .collect::<Vec<String>>()
                .join(" | ");

            paper.doi = f
                .pubmed_data
                .article_id_list
                .article_id
                .iter()
                .filter_map(|s| {
                    if &s.id_type[..] == "doi" {
                        Some(s.value.clone())
                    } else {
                        None
                    }
                })
                .collect::<Vec<String>>()
                .first()
                .unwrap_or(&format!(""))
                .to_string();

            paper.issn = f
                .medline_citation
                .article
                .journal
                .issn
                .clone()
                .unwrap_or("".to_string());
            if let Some(article_date) = f.medline_citation.article.article_date.as_ref() {
                if &article_date.date_type[..] == "Electronic" {
                    paper.epub_month = article_date.month.clone().unwrap_or(format!(""));
                    paper.epub_year = article_date.year.clone();
                }
            }

            let _ = paper.save_csv();
            paper
        })
        .collect::<Vec<PaperCsvResult>>();

    Ok(res)
}

fn url_encode(s: &str) -> String {
    let mut result = String::new();
    for ch in s.chars() {
        match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' | '~' | '+' => {
                result.push(ch);
            }
            ' ' => {
                result.push('+');
            }
            _ => {
                result.push_str(&format!("%{:02X}", ch as u32));
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use crate::eutils::*;

    #[test]
    fn test_urlencode() {
        let s = "(Ankylosing spondylitis[Title/Abstract]) AND (\"2023/1/10\"[Date - Publication] : \"2023/2/10\"[Date - Publication])";
        println!("{}", url_encode(s));
        assert_eq!("%28Ankylosing+spondylitis%5BTitle%2FAbstract%5D%29+AND+%28\"2023%2F1%2F10\"%5BDate+-+Publication%5D+%3A+\"2023%2F2%2F10\"%5BDate+-+Publication%5D%29", url_encode(s))
    }

    #[tokio::test]
    async fn test_esearch() {
        crate::config::init_config();

        let s = "(Ankylosing spondylitis[Title/Abstract]) AND (\"2023/1/10\"[Date - Publication] : \"2023/2/10\"[Date - Publication])";

        let result = esearch("pubmed", s).await;

        log::info!("result = {:?}", result);
    }

    #[tokio::test]
    async fn test_esearch2() {
        crate::config::init_config();

        let s = "(Ankylosing spondylitis[Title/Abstract]) AND (\"2023/1/10\"[Date - Publication] : \"2023/2/10\"[Date - Publication])";

        let result = esearch2("pubmed", s, Some(2), Some(1)).await;

        log::info!("result = {:?}", result);
    }

    #[tokio::test]
    async fn test_efetch() {
        crate::config::init_config();

        let ids = vec!["28250621"];

        let ii = ids.iter().map(|f| f.to_string()).collect::<Vec<String>>();
        let result = efetch("pubmed", &ii).await;

        log::info!("result = {:?}", result);
    }

    #[test]
    fn test_parse_xml() {
        crate::config::init_config();
        let str = r#"
        <PubmedArticleSet>
    <PubmedArticle>
        <MedlineCitation Status="MEDLINE" Owner="NLM" IndexingMethod="Automated">
            <PMID Version="1">36765305</PMID>
            <Article PubModel="Electronic">
                <Journal>
                    <ISSN IssnType="Electronic">1741-7015</ISSN>
                    <JournalIssue CitedMedium="Internet">
                        <PubDate>
                            <Year>2023</Year>
                            <Month>Feb</Month>
                            <Day>10</Day>
                        </PubDate>
                    </JournalIssue>
                    <Title>BMC medicine</Title>
                    <ISOAbbreviation>BMC Med</ISOAbbreviation>
                </Journal>
                <ArticleTitle>Dual-specificity phosphatases 22-deficient T cells contribute to the pathogenesis of ankylosing spondylitis.</ArticleTitle>
                <ELocationID EIdType="pii" ValidYN="Y">46</ELocationID>
                <ELocationID EIdType="doi" ValidYN="Y">10.1186/s12916-023-02745-6</ELocationID>
                <Abstract>
                    <AbstractText Label="BACKGROUND" NlmCategory="BACKGROUND">Dual-specificity phosphatases (DUSPs) can dephosphorylate both tyrosine and serine/threonine residues of their substrates and regulate T cell-mediated immunity and autoimmunity. The aim of this study was to investigate the potential roles of DUSPs in ankylosing spondylitis (AS).</AbstractText>
                    <AbstractText Label="METHODS" NlmCategory="METHODS">Sixty AS patients and 45 healthy controls were enrolled in this study. Associations of gene expression of 23 DUSPs in peripheral T cells with inflammatory cytokine gene expression and disease activity of AS were analyzed. Finally, we investigated whether the characteristics of AS are developed in DUSP-knockout mice.</AbstractText>
                    <AbstractText Label="RESULTS" NlmCategory="RESULTS">The mRNA levels of DUSP4, DUSP5, DUSP6, DUSP7, and DUSP14 in peripheral T cells were significantly higher in AS group than those of healthy controls (all p &lt; 0.05), while DUSP22 (also named JKAP) mRNA levels were significantly lower in AS group than healthy controls (p &lt; 0.001). The mRNA levels of DUSP4, DUSP5, DUSP6, DUSP7, and DUSP14 in T cells were positively correlated with mRNA levels of tumor necrosis factor-&#x3b1; (TNF-&#x3b1;), whereas DUSP22 was inversely correlated (all p &lt; 0.05). In addition, inverse correlations of DUSP22 gene expression in peripheral T cells with C-reactive protein, erythrocyte sedimentation rate, and Bath Ankylosing Spondylitis Disease Activity Index (BASDAI) were observed (all p &lt; 0.05). More importantly, aged DUSP22 knockout mice spontaneously developed syndesmophyte formation, which was accompanied by an increase of TNF-&#x3b1;<sup>+</sup>, interleukin-17A<sup>+</sup>, and interferon-&#x3b3;<sup>+</sup> CD3<sup>+</sup> T cells.</AbstractText>
                    <AbstractText Label="CONCLUSIONS" NlmCategory="CONCLUSIONS">DUSP22 may play a crucial role in the pathogenesis and regulation of disease activity of AS.</AbstractText>
                    <CopyrightInformation>&#xa9; 2023. The Author(s).</CopyrightInformation>
                </Abstract>
                <AuthorList CompleteYN="Y">
                    <Author ValidYN="Y" EqualContrib="Y">
                        <LastName>Chen</LastName>
                        <ForeName>Ming-Han</ForeName>
                        <Initials>MH</Initials>
                    </Author>
                    <Author ValidYN="Y" EqualContrib="Y">
                        <LastName>Chuang</LastName>
                        <ForeName>Huai-Chia</ForeName>
                        <Initials>HC</Initials>
                    </Author>
                    <Author ValidYN="Y">
                        <LastName>Yeh</LastName>
                        <ForeName>Yi-Chen</ForeName>
                        <Initials>YC</Initials>
                    </Author>
                    <Author ValidYN="Y">
                        <LastName>Chou</LastName>
                        <ForeName>Chung-Tei</ForeName>
                        <Initials>CT</Initials>
                    </Author>
                    <Author ValidYN="Y">
                        <LastName>Tan</LastName>
                        <ForeName>Tse-Hua</ForeName>
                        <Initials>TH</Initials>
                    </Author>
                </AuthorList>
                <PublicationTypeList>
                    <PublicationType UI="D016428">Journal Article</PublicationType>
                </PublicationTypeList>
                <ArticleDate DateType="Electronic">
                    <Year>2023</Year>
                    <Month>02</Month>
                    <Day>10</Day>
                </ArticleDate>
            </Article>
            <CitationSubset>IM</CitationSubset>
        </MedlineCitation>
        <PubmedData>
            <ArticleIdList>
                <ArticleId IdType="pubmed">36765305</ArticleId>
                <ArticleId IdType="pmc">PMC9921195</ArticleId>
                <ArticleId IdType="doi">10.1186/s12916-023-02745-6</ArticleId>
                <ArticleId IdType="pii">10.1186/s12916-023-02745-6</ArticleId>
            </ArticleIdList>
        </PubmedData>
    </PubmedArticle>
</PubmedArticleSet>"#;

        let p = parse_xml(str);

        match p {
            Ok(q) => log::info!("xml struct = {}", serde_json::to_string_pretty(&q).unwrap()),
            Err(e) => log::info!("xml struct error = {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_par() {
        crate::config::init_config();
        let urls = vec![
            "https://www.rust-lang.org",
            "https://www.google.com",
            "https://www.wikipedia.org",
            "https://github.com",
            "https://www.baidu.com",
            "https://stackoverflow.com",
            "https://gitlab.com",
            "https://www.microsoft.com",
            "https://www.mozilla.org",
            "https://www.cloudflare.com",
        ];

        async fn fetch_url(url: &str) {
            let id = lock().await;
            log::info!("request_task_id = {},  esearch, url = {}", id, &url);
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            unlock(id);
        }

        let mut tasks = vec![];
        for url in urls {
            tasks.push(tokio::spawn(fetch_url(url)));
        }

        for task in tasks {
            let _ = task.await;
        }
    }

    #[test]
    fn test_regex() {
        let input = r#"                <Abstract>
        <AbstractText>The dissemination of methicillin-resistant (MR) <i>Staphylococcus aureus</i> (SA) in community and health-care settings is of great concern and associated with high mortality and morbidity. Rapid detection of MRSA with short turnaround time can minimize the time to initiate appropriate therapy and further promote infection control. Early detection of MRSA directly from clinical samples is complicated by the frequent association of MRSA with methicillin-susceptible SA (MSSA) and coagulase-negative <i>Staphylococcus</i> (CoNS) species. Infection associated with true MRSA or MSSA is differentiated from CoNS, requires target specific primers for the presence of SA and <i>mec</i> A or <i>nuc</i> or <i>fem</i> A gene for confirmation of MR. Recently, livestock-associated MRSA carrying <i>mec</i> C variant complicates the epidemiology of MRSA further. Several commercial rapid molecular kits are available with a different combination of these targets for the detection of MRSA or MSSA. The claimed sensitivity and specificity of the currently available commercial kits is varying, because of the different target combination used for detection of SA and MR.</AbstractText>
    </Abstract>"#;
        let output = remove_str(input, "AbstractText");

        println!("{}", output);
    }
}
