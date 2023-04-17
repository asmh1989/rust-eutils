use std::{fs, os::unix::prelude::MetadataExt, path::Path};

use chrono::Datelike;
use csv::{QuoteStyle, WriterBuilder};
use rocket::serde::DeserializeOwned;
use serde::Serialize;

pub fn get_pmid_path_by_id(id: usize) -> String {
    let million: usize = 1000000;
    let thousand: usize = 1000;

    let first = id / million;

    let second = (id - first * million) / thousand;

    return format!(
        "data/pmid/{}/{}/{}.csv",
        (first + 1) * million,
        (second + 1) * thousand,
        id
    );
}

pub fn get_download_path_by_time(file_type: &str, id: i64) -> String {
    let date = chrono::NaiveDateTime::from_timestamp_millis(id);
    if let Some(d) = date {
        let year = d.year();
        let month = d.month();
        let day = d.day();

        return format!(
            "data/download_{}/{}-{}-{}/{}.{}",
            file_type, year, month, day, id, file_type
        );
    } else {
        return format!("not_found");
    }
}

pub fn get_download_path(file_type: &str) -> std::io::Result<(String, i64)> {
    let now = chrono::Utc::now().timestamp_millis();
    let file_name = get_download_path_by_time(file_type, now);

    let path = std::path::Path::new(&file_name);
    let prefix = path.parent().unwrap();
    std::fs::create_dir_all(prefix)?;

    Ok((file_name, now))
}

pub fn file_exist(path: &str) -> bool {
    let meta = fs::metadata(path);
    if let Ok(m) = meta {
        if m.is_file() && m.size() > 128 {
            return true;
        } else {
            if m.is_dir() {
                let _ = fs::remove_dir_all(path);
            }
            return false;
        }
    } else {
        return false;
    }
}

pub fn read_target_csv<P: AsRef<Path>, T: DeserializeOwned>(
    path: P,
    delimiter: u8,
    v: &mut Vec<T>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // let file = File::open(path)?;
    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(delimiter)
        .has_headers(true)
        .from_path(path)?;
    for result in rdr.deserialize() {
        let ele: T = result?;
        v.push(ele);
    }

    Ok(())
}

pub fn save_to_file<T: Serialize>(
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

#[cfg(test)]
mod tests {
    use log::info;

    use crate::model::PaperCsvResult;

    use super::*;

    #[test]
    fn test_parse_csv() {
        crate::config::init_config();
        let path = "data/pmid/37000000/753000/36752498.csv";
        let mut vec: Vec<PaperCsvResult> = Vec::new();
        let result = read_target_csv(path, b',', &mut vec);

        info!("result = {:?}", result);
        info!("csv = {}", serde_json::to_string_pretty(&vec).unwrap());
    }
}
