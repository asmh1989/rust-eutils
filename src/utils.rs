use std::{fs, os::unix::prelude::MetadataExt, path::Path};

use crate::model::PaperCsvResult;

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

pub fn read_target_csv<P: AsRef<Path>>(
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

#[cfg(test)]
mod tests {
    use log::info;

    use super::*;

    #[test]
    fn test_parse_csv() {
        crate::config::init_config();
        let path = "data/pmid/37000000/753000/36752498.csv";
        let mut vec = Vec::new();
        let result = read_target_csv(path, &mut vec);

        info!("result = {:?}", result);
        info!("csv = {}", serde_json::to_string_pretty(&vec).unwrap());
    }
}
