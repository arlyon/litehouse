#![deny(clippy::all)]

#[macro_use]
extern crate napi_derive;

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

impl From<Entry> for IntoEntry {
    fn from(val: Entry) -> Self {
        IntoEntry {
            title: val.title,
            version: (val.version[0], val.version[1], val.version[2]),
            description: val.description,
            schema: val.schema,
            sha: val.sha,
            size: val.size,
            capabilities: val.capabilities,
        }
    }
}

impl<'a> From<litehouse_registry::proto::litehouse::Entry<'a>> for Entry {
    fn from(val: litehouse_registry::proto::litehouse::Entry<'a>) -> Entry {
        Entry {
            title: val.title().map(|s| s.to_string()).unwrap_or_default(),
            version: val
                .version()
                .map(|v| vec![v.major(), v.minor(), v.patch()])
                .unwrap(),
            description: val.description().map(|s| s.to_string()).unwrap_or_default(),
            capabilities: vec![],
            schema: val.schema().map(|s| s.to_string()).unwrap_or_default(),
            size: val.size_(),
            sha: val.sha().map(|s| s.to_string()).unwrap_or_default(),
        }
    }
}

#[napi(js_name = "LitehouseRegistry")]
pub struct JsLitehouseRegistry(
    &'static LitehouseRegistry<'static>,
    &'static tokio::runtime::Runtime,
);

static ONCE: OnceCell<LitehouseRegistry<'static>> = OnceCell::const_new();
static RT: std::sync::OnceLock<tokio::runtime::Runtime> = std::sync::OnceLock::new();

#[napi]
impl JsLitehouseRegistry {
    #[napi(factory)]
    pub async fn global() -> Self {
        let reg = ONCE
            .get_or_init(|| async { LitehouseRegistry::new().await })
            .await;
        let rt = RT.get_or_init(|| tokio::runtime::Runtime::new().unwrap());
        Self(reg, rt)
    }

    #[napi]
    pub fn get(&self, title: String) -> Vec<Entry> {
        let rt = self.1;
        let reg = self.0;
        rt.block_on(async move { reg.get(title).await.into_iter().map(Into::into).collect() })
    }

    #[napi]
    pub fn insert(&self, entry: Entry) -> Option<()> {
        let rt = self.1;
        let reg = self.0;
        rt.block_on(async { reg.insert(entry.into()).map(|f| f.ok()).await })
    }

    #[napi]
    pub fn get_exact(&self, title: String, version: (u16, u16, u16)) -> Option<Entry> {
        let rt = self.1;
        let reg = self.0;
        rt.block_on(async { reg.get_exact(&title, version).await.map(Into::into) })
    }
}
