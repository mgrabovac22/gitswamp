use serde::{Deserialize, Serialize};

use super::DiffLine;

#[derive(Default, Serialize)]
pub struct ChangePreview {
    pub(crate) lines: Vec<DiffLine>,
    pub(crate) limited: bool,
    pub(crate) binary: bool,
}

#[derive(Serialize)]
pub(crate) struct Unit {
    pub(crate) id: usize,
    pub(crate) changes: Vec<usize>,
}

#[derive(Serialize)]
pub(crate) struct Group {
    pub(crate) subject: String,
    pub(crate) description: String,
    pub(crate) units: Vec<usize>,
}

#[derive(Deserialize)]
pub struct Draft {
    pub(crate) subject: String,
    pub(crate) description: String,
    pub(crate) units: Vec<usize>,
}
