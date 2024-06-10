#![deny(clippy::all)]

#[macro_use]
extern crate napi_derive;

use std::future::Future;

use futures::FutureExt;
use litehouse_registry::{partition::IntoEntry, LitehouseRegistry};
use tokio::sync::OnceCell;

#[napi(object)]
pub struct Entry {
    pub title: String,
    pub version: Vec<u16>,
    pub description: String,
    pub capabilities: Vec<String>,
    pub schema: String,
    pub size: u32,
    pub sha: String,
}

impl Into<IntoEntry> for Entry {
    fn into(self) -> IntoEntry {
        IntoEntry {
            title: self.title,
            version: (self.version[0], self.version[1], self.version[2]),
            description: self.description,
            schema: self.schema,
            sha: self.sha,
            size: self.size,
            capabilities: self.capabilities,
        }
    }
}

impl<'a> Into<Entry> for litehouse_registry::proto::litehouse::Entry<'a> {
    fn into(self) -> Entry {
        Entry {
            title: self.title().map(|s| s.to_string()).unwrap_or_default(),
            version: self
                .version()
                .map(|v| vec![v.major(), v.minor(), v.patch()])
                .unwrap(),
            description: self
                .description()
                .map(|s| s.to_string())
                .unwrap_or_default(),
            capabilities: vec![],
            schema: self.schema().map(|s| s.to_string()).unwrap_or_default(),
            size: self.size_(),
            sha: self.sha().map(|s| s.to_string()).unwrap_or_default(),
        }
    }
}

#[napi(js_name = "LitehouseRegistry")]
pub struct JsLitehouseRegistry(&'static LitehouseRegistry<'static>);

static ONCE: OnceCell<LitehouseRegistry<'static>> = OnceCell::const_new();

#[napi]
impl JsLitehouseRegistry {
    #[napi(constructor)]
    pub async fn new() -> Self {
        let reg = ONCE
            .get_or_init(|| async { LitehouseRegistry::new().await })
            .await;
        Self(reg)
    }

    #[napi]
    pub async fn insert(&self, entry: Entry) -> Option<()> {
        // self.0.insert(entry.into()).map(|f| f.ok()).await
        todo!()
    }

    #[napi]
    pub async fn get(&'static self, title: String) -> Vec<Entry> {
        self.0
            .get(&title)
            .await
            .into_iter()
            .map(Into::into)
            .collect()
    }

    #[napi]
    pub async fn get_prefix(&'static self, prefix: String) -> Vec<Entry> {
        self.0
            .get_prefix(&prefix)
            .await
            .into_iter()
            .map(Into::into)
            .collect()
    }

    pub async fn get_exact(&'static self, title: String, version: (u8, u8, u8)) -> Option<Entry> {
        self.0.get_exact(&title, version).await.map(Into::into)
    }
}
