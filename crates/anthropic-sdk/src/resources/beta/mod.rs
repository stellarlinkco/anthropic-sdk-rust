pub mod files;
pub mod messages;
pub mod models;

use crate::client::Inner;
use std::sync::Arc;

#[derive(Clone)]
pub struct Beta {
    pub messages: messages::Messages,
    pub models: models::Models,
    pub files: files::Files,
}

impl Beta {
    pub(crate) fn new(inner: Arc<Inner>) -> Self {
        Self {
            messages: messages::Messages::new(inner.clone()),
            models: models::Models::new(inner.clone()),
            files: files::Files::new(inner),
        }
    }
}
