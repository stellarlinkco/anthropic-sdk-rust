use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PageParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub before_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub after_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound(deserialize = "T: Deserialize<'de>"))]
pub struct Page<T> {
    #[serde(default)]
    pub data: Vec<T>,

    #[serde(default)]
    pub has_more: bool,

    pub first_id: Option<String>,
    pub last_id: Option<String>,
}

impl<T> Page<T> {
    pub fn has_next_page(&self) -> bool {
        if !self.has_more {
            return false;
        }
        if self.data.is_empty() {
            return false;
        }
        self.first_id.is_some() || self.last_id.is_some()
    }

    pub fn next_params(&self, original: &PageParams) -> Option<PageParams> {
        if !self.has_next_page() {
            return None;
        }

        if original.before_id.is_some() {
            let first_id = self.first_id.clone()?;
            return Some(PageParams {
                limit: original.limit,
                before_id: Some(first_id),
                after_id: None,
            });
        }

        let last_id = self.last_id.clone()?;
        Some(PageParams {
            limit: original.limit,
            before_id: None,
            after_id: Some(last_id),
        })
    }
}
