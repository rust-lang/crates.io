use crate::Crate;
use std::io::Write;

fn write_crate<W: Write>(krate: &Crate, mut writer: W) -> anyhow::Result<()> {
    serde_json::to_writer(&mut writer, krate)?;
    writer.write_all(b"\n")?;
    Ok(())
}

pub fn write_crates<W: Write>(crates: &[Crate], mut writer: W) -> anyhow::Result<()> {
    for krate in crates {
        write_crate(krate, &mut writer)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::*;

    #[test]
    fn test_write_crate() {
        let krate = Crate {
            name: "foo".to_string(),
            vers: "1.2.3".to_string(),
            deps: vec![],
            cksum: "0123456789asbcdef".to_string(),
            features: Default::default(),
            features2: None,
            yanked: None,
            links: None,
            rust_version: None,
            v: None,
        };
        let mut buffer = Vec::new();
        assert_ok!(write_crate(&krate, &mut buffer));
        assert_ok_eq!(
            String::from_utf8(buffer),
            "\
            {\"name\":\"foo\",\"vers\":\"1.2.3\",\"deps\":[],\"cksum\":\"0123456789asbcdef\",\"features\":{},\"yanked\":null}\n\
        "
        );
    }

    #[test]
    fn test_write_crates() {
        let versions = vec!["0.1.0", "1.0.0-beta.1", "1.0.0", "1.2.3"];
        let crates = versions
            .into_iter()
            .map(|vers| Crate {
                name: "foo".to_string(),
                vers: vers.to_string(),
                deps: vec![],
                cksum: "0123456789asbcdef".to_string(),
                features: Default::default(),
                features2: None,
                yanked: None,
                links: None,
                rust_version: None,
                v: None,
            })
            .collect::<Vec<_>>();

        let mut buffer = Vec::new();
        assert_ok!(write_crates(&crates, &mut buffer));
        assert_ok_eq!(
            String::from_utf8(buffer),
            "\
            {\"name\":\"foo\",\"vers\":\"0.1.0\",\"deps\":[],\"cksum\":\"0123456789asbcdef\",\"features\":{},\"yanked\":null}\n\
            {\"name\":\"foo\",\"vers\":\"1.0.0-beta.1\",\"deps\":[],\"cksum\":\"0123456789asbcdef\",\"features\":{},\"yanked\":null}\n\
            {\"name\":\"foo\",\"vers\":\"1.0.0\",\"deps\":[],\"cksum\":\"0123456789asbcdef\",\"features\":{},\"yanked\":null}\n\
            {\"name\":\"foo\",\"vers\":\"1.2.3\",\"deps\":[],\"cksum\":\"0123456789asbcdef\",\"features\":{},\"yanked\":null}\n\
        "
        );
    }
}
