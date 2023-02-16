use std::{
    path::Path,
    sync::{Arc, Mutex},
};

use crossbeam_deque::Worker;
use once_cell::sync::OnceCell;
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::{
    model::{PaperCsvResult, PubmedArticleSet},
    utils::{file_exist, get_pmid_path_by_id},
};

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
    translationset: Vec<String>,
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

pub async fn esearch(db: &str, query: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut retstart = 0;
    let page_size = PAGE_SIZE;

    let mut ids: Vec<String> = Vec::new();

    loop {
        let url = format!(
            "{}db={}&term={}&retmode=json&api_key=f6bc4f0e30a718d326ef842054d988ecdd08&retstart={}&retmax={}",
            ESEARCH,
            &db,
            &url_encode(&query),
            retstart,
            page_size
        );

        let id = lock().await;
        log::info!("request_task_id = {},  esearch, url = {}", id, &url);
        let resp = reqwest::get(&url).await?.json::<SearchResult>().await?;
        unlock(id);

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

    Ok(())
}

fn read_target_csv<P: AsRef<Path>>(
    path: P,
    v: &mut Vec<PaperCsvResult>,
) -> Result<(), Box<dyn std::error::Error>> {
    // let file = File::open(path)?;
    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(b',')
        .has_headers(true)
        .from_path(path)?;
    for result in rdr.deserialize() {
        let ele: PaperCsvResult = result?;
        v.push(ele);
    }

    Ok(())
}

pub async fn efetch(
    db: &str,
    ids: &Vec<String>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
            log::info!("pmid = {} already downloaded", pmid);

            let result = read_target_csv(&path, &mut v);
            if result.is_err() {
                log::warn!("pmid = {},  csv parse error = {:?}", pmid, result);
                let _ = std::fs::remove_file(&path);
            }
        }
    }

    log::info!("PaperCsvResult len = {:?}", v);

    Ok(())
}

fn parse_xml(xml: &str) -> Result<Vec<PaperCsvResult>, Box<dyn std::error::Error + Send + Sync>> {
    let re = Regex::new(r#"<sup>.*?</sup>"#).unwrap();
    let text = re.replace_all(xml, "").to_string();

    let p: PubmedArticleSet = serde_xml_rs::from_str(&text)?;

    let res = p
        .pubmed_article
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
                .clone();
            paper.journal_title = f.medline_citation.article.journal.title.clone();
            paper.journal_abbr = f.medline_citation.article.journal.iso_abbreviation.clone();
            if f.medline_citation.article.r#abstract.is_some() {
                paper.r#abstract = f
                    .medline_citation
                    .article
                    .r#abstract
                    .as_ref()
                    .unwrap()
                    .abstract_text
                    .iter()
                    .map(|v| v.value.clone())
                    .collect::<Vec<String>>()
                    .join(" ");
            }
            let authors = &f.medline_citation.article.author_list.authors;
            paper.author_first = format!(
                "{} {}",
                authors.first().unwrap().fore_name,
                authors.first().unwrap().last_name
            );

            if authors.len() > 1 {
                paper.author_last = format!(
                    "{} {}",
                    authors.last().unwrap().fore_name,
                    authors.last().unwrap().last_name
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
                .unwrap()
                .to_string();

            paper.issn = f.medline_citation.article.journal.issn.clone();
            if let Some(article_date) = f.medline_citation.article.article_date.as_ref() {
                if &article_date.date_type[..] == "Electronic" {
                    paper.epub_month = article_date.month.clone();
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
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' | '~' => {
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
    async fn test_efetch() {
        crate::config::init_config();

        let ids = vec!["36778985", "36774858"];

        let ii = ids.iter().map(|f| f.to_string()).collect::<Vec<String>>();
        let result = efetch("pubmed", &ii).await;

        log::info!("result = {:?}", result);
    }

    #[test]
    fn test_parse_xml() {
        crate::config::init_config();
        let str = r#"
        <?xml version="1.0" ?>
        <!DOCTYPE PubmedArticleSet PUBLIC "-//NLM//DTD PubMedArticle, 1st January 2023//EN" "https://dtd.nlm.nih.gov/ncbi/pubmed/out/pubmed_230101.dtd">
        <PubmedArticleSet>
            <PubmedArticle>
                <MedlineCitation Status="Publisher" Owner="NLM">
                    <PMID Version="1">36774858</PMID>
                    <DateRevised>
                        <Year>2023</Year>
                        <Month>02</Month>
                        <Day>12</Day>
                    </DateRevised>
                    <Article PubModel="Print-Electronic">
                        <Journal>
                            <ISSN IssnType="Electronic">1878-1705</ISSN>
                            <JournalIssue CitedMedium="Internet">
                                <Volume>116</Volume>
                                <PubDate>
                                    <Year>2023</Year>
                                    <Month>Feb</Month>
                                    <Day>10</Day>
                                </PubDate>
                            </JournalIssue>
                            <Title>International immunopharmacology</Title>
                            <ISOAbbreviation>Int Immunopharmacol</ISOAbbreviation>
                        </Journal>
                        <ArticleTitle>Purine metabolites promote ectopic new bone formation in ankylosing spondylitis.</ArticleTitle>
                        <Abstract>
                            <AbstractText>Ankylosing spondylitis (AS) is a chronic inflammatory rheumatic disease that mainly affects the axial skeleton, whose typical features are inflammatory back pain, bone structural damage and pathological new bone formation. The pathology of ectopic new bone formation is still little known. In this study, we found increased purine metabolites in plasma of patients with AS. Similarly, metabolome analysis indicated increased purine metabolites in both serum of CD4-Cre; Ptpn11 and SHP2-deficient chondrocytes. SHP2-deficient chondrocytes promoted the growth of wild type chondrocytes and differentiation of osteoblasts in CD4-Cre; Ptpn11<sup>fl/fl</sup> mice, which spontaneously developed AS-like bone disease. Purine metabolites, along with PTHrP derived from SHP2-deficient chondrocytes, accelerated the growth of chondrocytes and ectopic new bone formation through PKA/CREB signaling. Moreover, Suramin, a purinergic receptor antagonist, suppressed pathological new bone formation in AS-like bone disease. Overall, these results highlight the potential role of targeting purinergic signaling in retarding ectopic new bone formation in AS.</AbstractText>
                            <CopyrightInformation>Copyright &#xa9; 2023 Elsevier B.V. All rights reserved.</CopyrightInformation>
                        </Abstract>
                        <AuthorList CompleteYN="Y">
                            <Author ValidYN="Y">
                                <LastName>Zhang</LastName>
                                <ForeName>Shuqiong</ForeName>
                                <Initials>S</Initials>
                                <AffiliationInfo>
                                    <Affiliation>State Key Laboratory of Pharmaceutical Biotechnology, Department of Biotechnology and Pharmaceutical Sciences, School of Life Sciences, Nanjing University, 163 Xianlin Avenue, Nanjing 210023, China.</Affiliation>
                                </AffiliationInfo>
                            </Author>>
                            <Author ValidYN="Y">
                                <LastName>Shao</LastName>
                                <ForeName>Fenli</ForeName>
                                <Initials>F</Initials>
                                <AffiliationInfo>
                                    <Affiliation>State Key Laboratory of Pharmaceutical Biotechnology, Department of Biotechnology and Pharmaceutical Sciences, School of Life Sciences, Nanjing University, 163 Xianlin Avenue, Nanjing 210023, China; College of Pharmacy, Nanjing University of Chinese Medicine, Nanjing 210023, China. Electronic address: shaofenli90@163.com.</Affiliation>
                                </AffiliationInfo>
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
                </MedlineCitation>
                <PubmedData>
                    <ArticleIdList>
                        <ArticleId IdType="pubmed">36774858</ArticleId>
                        <ArticleId IdType="doi">10.1016/j.intimp.2023.109810</ArticleId>
                        <ArticleId IdType="pii">S1567-5769(23)00133-9</ArticleId>
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
}
