use serde::{Deserialize, Serialize};
use std::io::Write;
use std::{fs::File, io};

use crate::utils::{file_exist, get_download_path, get_pmid_path_by_id};

#[derive(Debug, Serialize, Deserialize)]
pub struct GeneDisease {
    pub disease: String,
    pub gene: String,
    pub last_ref_year_mining: Option<usize>,
    pub n_pubmed_minging: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PubmedArticleSet {
    pub pubmed_article: Option<Vec<PubmedArticle>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PubmedArticle {
    pub medline_citation: MedlineCitation,
    pub pubmed_data: PubmedData,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct MedlineCitation {
    #[serde(rename = "PMID")]
    pub pmid: String,
    pub article: Article,
    #[serde(rename = "Status")]
    pub status: String,
    #[serde(rename = "Owner")]
    pub owner: String,
    // #[serde(rename = "IndexingMethod")]
    // pub indexing_method: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Article {
    pub journal: Journal,
    pub article_title: String,
    pub r#abstract: Option<Abstract>,
    pub author_list: AuthorList,
    pub publication_type_list: PublicationTypeList,
    #[serde(rename = "PubModel")]
    pub pub_model: String,
    #[serde(rename = "ArticleDate")]
    pub article_date: Option<ArticleDate>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Journal {
    #[serde(rename = "ISSN")]
    pub issn: Option<String>,
    pub journal_issue: JournalIssue,
    pub title: String,
    #[serde(rename = "ISOAbbreviation")]
    pub iso_abbreviation: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct JournalIssue {
    pub pub_date: PubDate,
    #[serde(rename = "CitedMedium")]
    pub cited_medium: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PubDate {
    pub year: String,
    pub month: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Abstract {
    #[serde(rename = "AbstractText")]
    pub abstract_text: Vec<AbstractText>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct AbstractText {
    #[serde(rename = "$value")]
    pub value: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct AuthorList {
    #[serde(rename = "CompleteYN")]
    pub complete_yn: String,
    #[serde(rename = "Author")]
    pub authors: Vec<Author>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct Author {
    // #[serde(rename = "ValidYN")]
    // pub valid_yn: String,
    pub last_name: Option<String>,
    pub fore_name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PublicationTypeList {
    #[serde(rename = "PublicationType")]
    pub publication_types: Vec<PublicationType>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PublicationType {
    // #[serde(rename = "UI")]
    // pub ui: String,
    #[serde(rename = "$value")]
    pub value: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ArticleDate {
    pub date_type: String,
    pub year: String,
    pub month: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PubmedData {
    pub article_id_list: ArticleIdList,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ArticleIdList {
    pub article_id: Vec<ArticleId>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ArticleId {
    pub id_type: String,
    #[serde(rename = "$value")]
    pub value: String,
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct PaperCsvResult {
    #[serde(rename = "PMID")]
    pub pmid: String,
    pub title: String,
    #[serde(rename = "PubDateYear")]
    pub pubdate_year: String,
    #[serde(rename = "PubDateMonth")]
    pub pubdate_month: String,
    pub journal_title: String,
    pub journal_abbr: String,
    pub r#abstract: String,
    pub author_first: String,
    pub author_last: String,
    pub publication_type: String,
    #[serde(rename = "DOI")]
    pub doi: String,
    #[serde(rename = "ISSN")]
    pub issn: String,
    pub epub_year: String,
    pub epub_month: String,
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct PaperCsvSummary {
    #[serde(rename = "PMID")]
    pub pmid: String,
    pub title: String,
    #[serde(rename = "PubDateYear")]
    pub pubdate_year: String,
    #[serde(rename = "PubDateMonth")]
    pub pubdate_month: String,
    pub journal_title: String,
    pub journal_abbr: String,
    pub r#abstract: String,
    pub author_first: String,
    pub author_last: String,
    pub publication_type: String,
    #[serde(rename = "DOI")]
    pub doi: String,
    #[serde(rename = "ISSN")]
    pub issn: String,
    pub epub_year: String,
    pub epub_month: String,
    pub summary: String,
}

impl PaperCsvResult {
    fn get_row_str(&self) -> String {
        return format!(
            "{:?},{:?},{:?},{:?},{:?},{:?},{:?},{:?},{:?},{:?},{:?},{:?},{:?},{:?}",
            self.pmid,
            self.title,
            self.pubdate_year,
            self.pubdate_month,
            self.journal_title,
            self.journal_abbr,
            self.r#abstract.replace("\"", ""),
            self.author_first,
            self.author_last,
            self.publication_type,
            self.doi,
            self.issn,
            self.epub_year,
            self.epub_month,
        )
        .replace("\n", "");
    }

    fn get_csv_header() -> String {
        format!("PMID,Title,PubDateYear,PubDateMonth,JournalTitle,JournalAbbr,Abstract,AuthorFirst,AuthorLast,PublicationType,DOI,ISSN,EpubYear,EpubMonth")
    }
    // 保存到 CSV 文件
    pub fn save_csv(&self) -> io::Result<()> {
        if self.pmid.is_empty() {
            return Ok(());
        }

        let id = self.pmid.parse::<usize>().unwrap();

        let file_name = get_pmid_path_by_id(id);

        if file_exist(&file_name) {
            return Ok(());
        }

        let path = std::path::Path::new(&file_name);
        let prefix = path.parent().unwrap();
        std::fs::create_dir_all(prefix)?;

        // 创建文件并写入标题行
        let mut file: File = File::create(file_name)?;
        writeln!(file, "{}", Self::get_csv_header())?;

        // 写入数据行
        let row = self.get_row_str();
        writeln!(file, "{}", row)?;

        Ok(())
    }

    pub fn save_list_csv(list: &Vec<PaperCsvResult>) -> io::Result<serde_json::Value> {
        let (file_name, now) = get_download_path("csv")?;

        // 创建文件并写入标题行
        let mut file: File = File::create(file_name)?;

        let header = Self::get_csv_header();
        let content = list
            .iter()
            .map(|f| f.get_row_str())
            .collect::<Vec<String>>();
        let c = format!("{}\n{}", header.as_str(), content.join("\n").as_str());

        writeln!(file, "{}", c)?;

        Ok(serde_json::json!({ "id": now }))
    }

    pub fn to_summary(&self, summary: String) -> PaperCsvSummary {
        PaperCsvSummary {
            pmid: self.pmid.clone(),
            title: self.title.clone(),
            pubdate_year: self.pubdate_year.clone(),
            pubdate_month: self.pubdate_month.clone(),
            journal_title: self.journal_title.clone(),
            journal_abbr: self.journal_abbr.clone(),
            r#abstract: self.r#abstract.clone(),
            author_first: self.author_first.clone(),
            author_last: self.author_last.clone(),
            publication_type: self.publication_type.clone(),
            doi: self.doi.clone(),
            issn: self.issn.clone(),
            epub_year: self.epub_year.clone(),
            epub_month: self.epub_month.clone(),
            summary,
        }
    }
}
