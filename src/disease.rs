use crate::{
    eutils::{efetch, fetch_ids},
    model::GeneDisease,
    utils::read_target_csv,
};

async fn query(g: &mut GeneDisease) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let query = format!(
        "({}[Title/Abstract]) AND ({}[Title/Abstract])",
        &g.gene, &g.disease
    );

    let mut ids: Vec<String> = Vec::new();
    let resp = fetch_ids("pubmed", &query, 0, 4).await?;
    ids.extend(resp.esearchresult.idlist.iter().cloned());
    let res = efetch("pubmed", &ids).await?;

    (*g).n_pubmed_minging = Some(resp.esearchresult.count.parse::<usize>().unwrap_or(0));
    if !res.is_empty() {
        (*g).last_ref_year_mining = Some(res[0].pubdate_year.parse::<usize>().unwrap_or(1970));
    }

    Ok(())
}

async fn query_gene_and_disease(
    path: &str,
) -> Result<Vec<GeneDisease>, Box<dyn std::error::Error + Send + Sync>> {
    let mut genes: Vec<GeneDisease> = Vec::new();
    let _ = read_target_csv(path, b',', &mut genes);

    for g in &mut genes {
        query(g).await?;
    }
    Ok(genes)
}

#[cfg(test)]
mod tests {
    use crate::utils::save_to_file;

    use super::*;

    #[tokio::test]
    async fn test_query() {
        crate::config::init_config();
        let mut g = GeneDisease {
            gene: "TYK2".to_string(),
            disease: "SLE".to_string(),
            n_pubmed_minging: None,
            last_ref_year_mining: None,
        };

        let _ = query(&mut g).await;

        log::info!("result = {:?}", g);
    }

    #[tokio::test]
    async fn test_query_gene_and_disease() {
        crate::config::init_config();

        let rr = query_gene_and_disease("data/gene_disease.csv")
            .await
            .unwrap();

        let _ = save_to_file("data/output.csv", &rr);
    }
}
