use std::collections::HashMap;
use url;

macro_rules! try_option( ($e:expr) => (
    match $e { Some(k) => k, None => return None }
) )

pub fn parse(s: &str) -> Option<HashMap<String, String>> {
    let mut map = HashMap::new();
    for part in s.split('&') {
        let mut i = part.splitn('=', 1);
        let key = try_option!(i.next());
        let value = try_option!(i.next());
        let key = key.replace("+", " ");
        let value = value.replace("+", " ");

        let key = try_option!(url::decode_component(key.as_slice()).ok());
        let value = try_option!(url::decode_component(value.as_slice()).ok());
        map.insert(key, value);
    }
    return Some(map)
}
