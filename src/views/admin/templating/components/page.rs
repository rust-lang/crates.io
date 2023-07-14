use crate::controllers::helpers::pagination::{self, Paginated};
use serde::{ser::SerializeMap, Serialize};

#[derive(Debug, Clone)]
pub struct Page {
    current: u32,
    total: u32,
    q: Option<String>,
}

impl Page {
    pub fn new<T>(resultset: &Paginated<T>, q: Option<&str>) -> Self {
        Self {
            current: if let pagination::Page::Numeric(n) = resultset.current_page() {
                *n
            } else {
                // We just don't support seek pagination right now in the admin, and if it's not
                // specific, we want to default to 1 regardless.
                1
            },
            total: resultset.total_pages() as u32,
            q: q.map(|s| s.to_string()),
        }
    }

    fn page_url(&self, page: u32) -> String {
        use url::form_urlencoded::Serializer;

        let mut s = Serializer::new(String::from("?"));
        s.append_pair("page", &page.to_string());
        if let Some(q) = &self.q {
            s.append_pair("q", q);
        }
        s.finish()
    }
}

impl Serialize for Page {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        #[derive(Serialize)]
        struct PageView {
            number: u32,
            active: bool,
            url: String,
        }

        let mut map = serializer.serialize_map(Some(4))?;
        map.serialize_entry("paginated", &(self.total > 1))?;
        map.serialize_entry(
            "previous",
            &if self.current < 2 {
                None
            } else {
                Some(self.page_url(self.current - 1))
            },
        )?;
        map.serialize_entry(
            "next",
            &if self.current >= self.total {
                None
            } else {
                Some(self.page_url(self.current + 1))
            },
        )?;
        map.serialize_entry(
            "pages",
            &((1..=self.total)
                .map(|page| PageView {
                    number: page,
                    active: page == self.current,
                    url: self.page_url(page),
                })
                .collect::<Vec<_>>()),
        )?;
        map.end()
    }
}
