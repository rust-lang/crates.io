//! This module contains a set of RSA keys that must be used only for
//! testing purposes. The keys are not secure and must not be used
//! in production!

use jsonwebtoken::errors::Error;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey};
use serde::Serialize;
use std::sync::LazyLock;

const PRIVATE_KEY_PEM: &[u8] = br#"
-----BEGIN PRIVATE KEY-----
MIIEvQIBADANBgkqhkiG9w0BAQEFAASCBKcwggSjAgEAAoIBAQCG7Za7JLYjNrWK
2dDl1Hg2lkBxBJSR4KfMVQ8PyN/hz6GGwTondSLTUVh9BzKfClnxcThqmv6awKZV
gv2ZV9FBUtOyromyZomdLRmYYzA8FLgXmXxqJqit8jQIAbpDGHz2qRdh4PITyEPl
Oib+hdhbQIOPK27xmPHXcQJ3gQoHoiCDkXbhDgztYM6BBKnSGaQPnW81p4pWEsFk
fhsmDKDm07EI/l96IQXbGlETna41+dtVmz83nL4DJ7jJUxuAlH8iH6w+4aMhjDmK
j7y4m394ceq0IWbptKZ7T/ewpKsbSzR14UuvBtLNeArSm5WqUmemxiWjeTo155Dp
c8tnMG6BAgMBAAECggEAf0tFCje/Ugd6TI3kRAAoja9BCp78n4eoJuEUfZrQhRRC
2oQPnkwnV+AFsKcKvfqhEmTzibfCfjNEeaZEJNgxxgQjTw7VP6b3K37yB8+EIRqW
90TJmMfyGXFIX0lp9YTz2C18rs3u9HTagTdUtImHrcd2lqquV2You02VuzLVSI7q
q4NvY1dHPRNo5g42EDhRVWjKPVq21EMGSsawSz/Y5jHoDIG4VRCqN1tuOwmKQw03
6ldStckDshRcb6pfFsrsfC0YHqXM2SSwS15C2NlEMIzVITKXaHCf5ole08+F5kmU
ADav+hHONHogKf2zsb2rd9khqbRgEEZl5ArbtudgwQKBgQDfUJYeLRZaDxhnlhP7
nfSWw0uYuXUu0LxsknHQMttC5YOKZRHl3RYHWSfNHMCe66geK5VykxSNOw2bVACm
hwJr4JZYgk3opLhvyvdRJ8NuoI4JTo24CoEF8EKHhJGPHMXQSre3JmK8ptPep9+P
/gTXT/U3Vlf9puZkoppq+/IEyQKBgQCarTptO3caJoCFlYnrCZ3X28LPv37tFjmE
AHRL5wxeFWhSZzemVul3v4vZvXgyc+VOBQoFvkQba7DncA1WVbl+zLgu8QZFotFf
VI3bZAK+02wqLXIo1CnAMB921Vn3UrHItToiCOJTHSEalxlTDEkSEMxx09sYGFh5
REIcQXIP+QKBgHW9zYiXiSNuthVXoa2WuLEMwz0A+3H1iINOK0f0qHp6/IHpjChA
Cy9QqJWSxVSFN5zAqglA1yMnsaLmBXnH0VUDkwGTonQ49S2sO/3EE1yutnTdwAb7
Ms/ov4soMH7eUsXhvz+Hs6N36lmI9Wy8J91GQSouEjKg3vTMbtJdiFtRAoGBAJYA
60mlwsKsljV2qXM0N0xwxoP8/ZXl2M+INUCrCJZxgmNv0EtTvEUykOkQU3HybW31
exvIwnopPT2lsHmK10L+PJzhiCieVxhxgsVCP1ta5Goe+rhX0UmeIdV34TD2lI3G
G2OIZB0ggcsswBWHM5H+kpbNU4wRiDPKm6aVXY3ZAoGAVJPyITFs2foRiRG1S8o9
gfz6rWleaXO2OmFh5P3UehhLwMr+vjvZn+8VByUubA9wqnY2JWu9ZSvdbdP6L6Z4
usn0CLeCS1Gdbk4piqiSmUAe7nt2Sh258SVG5deDX6ej06NQzy249TtufxXjZ/3y
68y3i6u6aIE4wCiMYXl9B0o=
-----END PRIVATE KEY-----
"#;

const PUBLIC_KEY_JWK: &str = r#"
{
    "kty": "RSA",
    "e": "AQAB",
    "use": "sig",
    "kid": "c0ffee",
    "alg": "RS256",
    "n": "hu2WuyS2Iza1itnQ5dR4NpZAcQSUkeCnzFUPD8jf4c-hhsE6J3Ui01FYfQcynwpZ8XE4apr-msCmVYL9mVfRQVLTsq6JsmaJnS0ZmGMwPBS4F5l8aiaorfI0CAG6Qxh89qkXYeDyE8hD5Tom_oXYW0CDjytu8Zjx13ECd4EKB6Igg5F24Q4M7WDOgQSp0hmkD51vNaeKVhLBZH4bJgyg5tOxCP5feiEF2xpRE52uNfnbVZs_N5y-Aye4yVMbgJR_Ih-sPuGjIYw5io-8uJt_eHHqtCFm6bSme0_3sKSrG0s0deFLrwbSzXgK0puVqlJnpsYlo3k6NeeQ6XPLZzBugQ"
}
"#;

pub(crate) const KEY_ID: &str = "c0ffee";

static ENCODING_KEY: LazyLock<EncodingKey> =
    LazyLock::new(|| EncodingKey::from_rsa_pem(PRIVATE_KEY_PEM).unwrap());

#[allow(unused)]
pub(crate) static DECODING_KEY: LazyLock<DecodingKey> = LazyLock::new(|| {
    let jwk = serde_json::from_str(PUBLIC_KEY_JWK).unwrap();
    DecodingKey::from_jwk(&jwk).unwrap()
});

pub fn encode_for_testing(claims: &impl Serialize) -> Result<String, Error> {
    let header = jsonwebtoken::Header {
        alg: Algorithm::RS256,
        kid: Some(KEY_ID.into()),
        ..Default::default()
    };

    jsonwebtoken::encode(&header, claims, &ENCODING_KEY)
}
