pub use chrono::NaiveDate;
pub use serde::{Deserialize, Serialize};
pub use serde_json;
pub use std::sync::Mutex;

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
