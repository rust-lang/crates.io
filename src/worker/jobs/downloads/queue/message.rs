use std::str::FromStr;

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Object {
    pub key: String,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bucket {
    pub name: String,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct S3 {
    pub bucket: Bucket,
    pub object: Object,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Record {
    #[serde(rename = "awsRegion")]
    pub aws_region: String,
    pub s3: S3,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Message {
    #[serde(rename = "Records")]
    pub records: Vec<Record>,
}

impl FromStr for Message {
    type Err = serde_json::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_str(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_debug_snapshot;

    #[test]
    fn test_parse() {
        let event = assert_ok!(include_str!("./fixtures/empty-event.json").parse::<Message>());
        assert_debug_snapshot!(event);

        let event = assert_ok!(include_str!("./fixtures/valid-event.json").parse::<Message>());
        assert_debug_snapshot!(event);

        let event = assert_ok!(include_str!("./fixtures/multi-event.json").parse::<Message>());
        assert_debug_snapshot!(event);
    }
}
