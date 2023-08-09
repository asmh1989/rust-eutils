use std::{sync::Arc, time::Duration};

use log::info;

use mongodb::{
    bson::{self, doc, Bson, Document},
    error::Error,
    options::{ClientOptions, FindOneOptions, FindOptions},
    Client, Cursor,
};
use once_cell::sync::OnceCell;
use rocket::futures::StreamExt;
use serde::de::DeserializeOwned;

static INSTANCE: OnceCell<Arc<Client>> = OnceCell::new();

pub const TABLE_NAME: &'static str = "cloud_monitor";
pub const COLLECTION_CID_NOT_FOUND: &'static str = "cid_not_found";

const KEY_UPDATE_TIME: &'static str = "updateTime";
const KEY_CREATE_TIME: &'static str = "createTime";

// #[macro_export]
// macro_rules! filter_cid {
//     ($e:expr) => {
//         mongodb::bson::doc! {"cid" : $e}
//     };
// }

#[derive(Clone, Debug)]
pub struct Db;

impl Db {
    pub fn get_instance() -> &'static Arc<Client> {
        INSTANCE.get().expect("db need init first")
    }

    pub async fn find<T>(
        table: &str,
        filter: impl Into<Option<Document>>,
        options: impl Into<Option<FindOptions>>,
        call_back: &dyn Fn(T),
    ) -> Result<(), Error>
    where
        T: DeserializeOwned,
    {
        let client = Db::get_instance();
        let db = client.database(TABLE_NAME);
        let collection = db.collection::<Document>(table);

        let mut cursor = collection.find(filter, options).await?;

        // Iterate over the results of the cursor.
        while let Some(result) = cursor.next().await {
            match result {
                Ok(document) => {
                    let result = bson::from_bson::<T>(Bson::Document(document));
                    match result {
                        Ok(app) => call_back(app),
                        Err(err) => {
                            info!("err = {:?}", err);
                        }
                    }
                }
                Err(e) => {
                    info!("error = {:?}", e);
                    return Err(e.into());
                }
            }
        }

        Ok(())
    }

    pub async fn find_one(
        table: &str,
        filter: impl Into<Option<Document>>,
        options: impl Into<Option<FindOneOptions>>,
    ) -> Result<Option<Document>, Error> {
        let client = Db::get_instance();
        let db = client.database(TABLE_NAME);
        let collection = db.collection(table);

        collection.find_one(filter, options).await
    }

    pub async fn find_one_with_table(
        table: &str,
        c: &str,
        filter: impl Into<Option<Document>>,
        options: impl Into<Option<FindOneOptions>>,
    ) -> Result<Option<Document>, Error> {
        let client = Db::get_instance();
        let db = client.database(table);
        let collection = db.collection(c);

        collection.find_one(filter, options).await
    }

    pub async fn find_with_table(
        table: &str,
        c: &str,
        filter: impl Into<Option<Document>>,
        options: impl Into<Option<FindOptions>>,
    ) -> Result<Cursor<Document>, Error> {
        let client = Db::get_instance();
        let db = client.database(table);
        let collection = db.collection::<Document>(c);

        collection.find(filter, options).await
    }

    pub async fn insert_many(table: &str, data: Vec<Document>) -> Result<(), Error> {
        let client = Db::get_instance();
        let db = client.database(TABLE_NAME);
        let collection = db.collection(table);
        let date = Bson::DateTime(mongodb::bson::DateTime::now());
        let data2: Vec<Document> = data
            .clone()
            .iter_mut()
            .map(|f| {
                f.insert(KEY_UPDATE_TIME, date.clone());
                f.insert(KEY_CREATE_TIME, date.clone());
                f.to_owned()
            })
            .collect();

        let _result = collection.insert_many(data2, None).await?;

        Ok(())
    }

    pub async fn delete_table(table: &str) -> Result<(), Error> {
        let client = Db::get_instance();
        let db = client.database(TABLE_NAME);
        let collection = db.collection::<Document>(table);
        let _ = collection.drop(None).await?;
        Ok(())
    }

    pub async fn save_with_table(
        table: &str,
        c: &str,
        filter: Document,
        app: Document,
    ) -> Result<(), Error> {
        let client = Db::get_instance();
        let db = client.database(table);
        let collection = db.collection(c);

        let mut update_doc = app;
        let date = Bson::DateTime(mongodb::bson::DateTime::now());
        update_doc.insert(KEY_UPDATE_TIME, date.clone());

        let result = collection.find_one(filter.clone(), None).await?;

        if !result.is_none() {
            // info!("db update: {:?}", filter.clone());
            collection
                .update_one(filter.clone(), doc! {"$set": update_doc}, None)
                .await?;
        } else {
            update_doc.insert(KEY_CREATE_TIME, date);
            let _ = collection.insert_one(update_doc, None).await?;

            // info!("db insert {:?}", filter.clone());
        }

        Ok(())
    }

    pub async fn insert_with_table(table: &str, c: &str, app: Document) -> Result<(), Error> {
        let client = Db::get_instance();
        let db = client.database(table);
        let collection = db.collection(c);

        let mut update_doc = app;
        let date = Bson::DateTime(mongodb::bson::DateTime::now());
        update_doc.insert(KEY_UPDATE_TIME, date.clone());
        update_doc.insert(KEY_CREATE_TIME, date);
        let _ = collection.insert_one(update_doc, None).await?;

        Ok(())
    }

    pub async fn save(c: &str, filter: Document, app: Document) -> Result<(), Error> {
        return Db::save_with_table(TABLE_NAME, c, filter, app).await;
    }

    pub async fn delete(table: &str, filter: Document) -> Result<(), Error> {
        let client = Db::get_instance();
        let db = client.database(TABLE_NAME);
        let collection = db.collection::<Document>(table);

        let result = collection.delete_one(filter, None).await?;

        info!("db delete {:?}", result);

        Ok(())
    }

    pub async fn contians(table: &str, filter: Document) -> bool {
        let client = Db::get_instance();
        let db = client.database(TABLE_NAME);
        let collection = db.collection::<Document>(table);

        let result = collection.count_documents(filter, None).await;

        match result {
            Ok(d) => d > 0,
            Err(_) => false,
        }
    }

    pub async fn count_with_table(table: &str, c: &str, filter: Document) -> u64 {
        let client = Db::get_instance();
        let db = client.database(table);
        let collection = db.collection::<Document>(c);

        let result = collection.count_documents(filter, None).await;

        match result {
            Ok(d) => d,
            Err(_) => 0,
        }
    }

    pub async fn count_with_table2(table: &str, c: &str) -> u64 {
        let client = Db::get_instance();
        let db = client.database(table);
        let collection = db.collection::<Document>(c);

        let result = collection.estimated_document_count(None).await;

        match result {
            Ok(d) => d,
            Err(_) => 0,
        }
    }

    pub async fn count(table: &str, filter: Document) -> u64 {
        Db::count_with_table(TABLE_NAME, table, filter).await
    }
}

pub async fn init_db(url: &str) {
    if INSTANCE.get().is_some() {
        return;
    }
    let mut client_options = ClientOptions::parse(url).await.unwrap();
    client_options.connect_timeout = Some(Duration::new(4, 0));
    // 选择超时
    client_options.server_selection_timeout = Some(Duration::new(8, 0));

    INSTANCE
        .set(Arc::new(Client::with_options(client_options).unwrap()))
        .expect("db init error");
}

#[cfg(test)]
mod tests {

    use super::*;

    #[tokio::test]
    async fn test_init() {
        crate::config::init_config();
        crate::slurm::init().await;
        log::info!(
            "smiles.count= {}",
            Db::count_with_table("AIXB", "smiles", doc! {}).await
        );
    }
}
