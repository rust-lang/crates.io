use std::collections::HashMap;
use url;

pub fn parse(s: &str) -> Option<HashMap<String, String>> {
    let mut map = HashMap::new();
    for part in s.split('&') {
        let mut i = part.splitn('=', 1);
        let key = match i.next() { Some(k) => k, None => return None };
        let value = match i.next() { Some(k) => k, None => return None };
        let key = key.replace("+", " ");
        let value = value.replace("+", " ");

        map.insert(url::decode_component(key.as_slice()),
                   url::decode_component(value.as_slice()));
    }
    return Some(map)
}
