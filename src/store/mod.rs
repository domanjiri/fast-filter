mod category;

use crate::context::Context;

use croaring::Bitmap;
use serde::{Deserialize, Serialize};
use tracing::{error, info};

use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Default)]
pub(crate) struct DataStore {
    pub ads: Vec<Arc<Ad>>,
    pub filters: Filters,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub(crate) struct Ad {
    pub id: String,
    pub categories: Vec<u32>,
    pub cities: Vec<u32>,
    pub hours: Vec<u32>,
}

/// Used for deserilize data, see `watcher.rs`
#[derive(Deserialize, Serialize, Debug, Default)]
pub(crate) struct Ads(pub Vec<Ad>);

#[derive(Debug, Default)]
pub(crate) struct Filters {
    pub all_pass: Bitmap,
    pub category: HashMap<u32, Bitmap>,
    pub city: Vec<Bitmap>, /* 1 - 10_000*/
    pub hour: Vec<Bitmap>, /* 0-23 */
}

impl DataStore {
    pub fn new() -> DataStore {
        Self {
            ads: Vec::with_capacity(32),
            filters: Filters {
                all_pass: Bitmap::new(),
                category: HashMap::with_capacity(10_000),
                city: vec![Bitmap::new(); 10_000],
                hour: vec![Bitmap::new(); 24],
            },
        }
    }

    pub async fn update(context: Context, ads: Vec<Arc<Ad>>) {
        // Bitmap's been implemented for u32
        let len = ads.len();
        if len >= u32::MAX as usize {
            error!(
                "num of ads({}) exceeded max allowed value({})",
                len,
                u32::MAX
            );
            return;
        }

        // All-pass filter
        let mut default_set = Bitmap::new();
        default_set.add_range(1..len as u32);

        let mut data = Self::new();
        data.ads = ads;
        data.filters.all_pass = default_set.clone();
        data.filters.city.fill(default_set.clone());
        data.filters.hour.fill(default_set.clone());

        for (index, ad) in data.ads.iter().enumerate() {
            // category
            let cats = if !ad.categories.is_empty() {
                ad.categories.clone()
            } else {
                category::CATEGORIES.to_vec()
            };
            for cat in cats.iter() {
                if let Some(value) = data.filters.category.get_mut(cat) {
                    value.add(index as u32);
                } else {
                    let mut value = default_set.clone();
                    value.add(index as u32);
                    data.filters.category.insert(cat.to_owned(), value);
                }
            }

            // city
            let cities = if !ad.cities.is_empty() {
                ad.cities.clone()
            } else {
                (0..10_000).collect::<Vec<u32>>()
            };
            for city in cities.iter() {
                if *city >= 10_000 {
                    continue;
                }
                if let Some(v) = data.filters.city.get_mut(*city as usize) {
                    v.add(index as u32);
                }
            }

            // hour
            let hours = if !ad.hours.is_empty() {
                ad.hours.clone()
            } else {
                (0..24).collect::<Vec<u32>>()
            };
            for hour in hours.iter() {
                if *hour >= 24 {
                    continue;
                }
                if let Some(v) = data.filters.hour.get_mut(*hour as usize) {
                    v.add(index as u32);
                }
            }
        }

        context.inventory.store(Arc::new(data));
        info!("Inventory's been updated");
    }
}
