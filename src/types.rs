pub use chrono::NaiveDate;
pub use serde::{Deserialize, Serialize};
pub use serde_json;
pub use std::sync::Mutex;

use lazy_static::lazy_static;

#[derive(Debug, Serialize, Deserialize)]
pub enum BagType {
    Black,
    Blue,
    GreenOrBrown,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CollectionDate {
    pub bag_type: BagType,
    pub date: NaiveDate,
}

lazy_static! {
    pub static ref handlebars_ref: handlebars::Handlebars<'static> = {
        let mut hb = handlebars::Handlebars::new();
        let mut options = handlebars::DirectorySourceOptions::default();
        options.tpl_extension = String::from(".html");
        hb.register_templates_directory("./html", options).unwrap();
        hb
    };
}
