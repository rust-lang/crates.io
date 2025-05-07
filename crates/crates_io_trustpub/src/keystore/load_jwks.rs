use jsonwebtoken::jwk::JwkSet;
use reqwest::Client;

/// Loads JSON Web Key Sets (JWKS) from an OpenID Connect provider.
///
/// This function implements the OpenID Connect Discovery process to fetch the JWKS:
///
/// 1. It first retrieves the OpenID configuration from the standard
///    `.well-known/openid-configuration` endpoint at the provided issuer URI.
/// 2. It extracts the `jwks_uri` from the configuration.
/// 3. It fetches the JWKS from the extracted URI.
///
/// The JWKS contains the public keys used to verify JWT signatures issued by the provider.
pub async fn load_jwks(client: &Client, issuer_uri: &str) -> reqwest::Result<JwkSet> {
    #[derive(Debug, serde::Deserialize)]
    struct OpenIdConfig {
        jwks_uri: String,
    }

    let url = format!("{issuer_uri}/.well-known/openid-configuration");
    let response = client.get(url).send().await?.error_for_status()?;
    let openid_config: OpenIdConfig = response.json().await?;

    let url = openid_config.jwks_uri;
    let response = client.get(url).send().await?.error_for_status()?;
    let jwks: JwkSet = response.json().await?;

    Ok(jwks)
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::assert_ok;
    use insta::assert_debug_snapshot;

    const GITHUB_JWKS: &str = r#"{
        "keys": [{
            "kty": "RSA",
            "alg": "RS256",
            "use": "sig",
            "kid": "cc413527-173f-5a05-976e-9c52b1d7b431",
            "n": "w4M936N3ZxNaEblcUoBm-xu0-V9JxNx5S7TmF0M3SBK-2bmDyAeDdeIOTcIVZHG-ZX9N9W0u1yWafgWewHrsz66BkxXq3bscvQUTAw7W3s6TEeYY7o9shPkFfOiU3x_KYgOo06SpiFdymwJflRs9cnbaU88i5fZJmUepUHVllP2tpPWTi-7UA3AdP3cdcCs5bnFfTRKzH2W0xqKsY_jIG95aQJRBDpbiesefjuyxcQnOv88j9tCKWzHpJzRKYjAUM6OPgN4HYnaSWrPJj1v41eEkFM1kORuj-GSH2qMVD02VklcqaerhQHIqM-RjeHsN7G05YtwYzomE5G-fZuwgvQ",
            "e": "AQAB"
        }, {
            "kty": "RSA",
            "alg": "RS256",
            "use": "sig",
            "kid": "38826b17-6a30-5f9b-b169-8beb8202f723",
            "n": "5Manmy-zwsk3wEftXNdKFZec4rSWENW4jTGevlvAcU9z3bgLBogQVvqYLtu9baVm2B3rfe5onadobq8po5UakJ0YsTiiEfXWdST7YI2Sdkvv-hOYMcZKYZ4dFvuSO1vQ2DgEkw_OZNiYI1S518MWEcNxnPU5u67zkawAGsLlmXNbOylgVfBRJrG8gj6scr-sBs4LaCa3kg5IuaCHe1pB-nSYHovGV_z0egE83C098FfwO1dNZBWeo4Obhb5Z-ZYFLJcZfngMY0zJnCVNmpHQWOgxfGikh3cwi4MYrFrbB4NTlxbrQ3bL-rGKR5X318veyDlo8Dyz2KWMobT4wB9U1Q",
            "e": "AQAB",
            "x5c": ["MIIDKzCCAhOgAwIBAgIUDnwm6eRIqGFA3o/P1oBrChvx/nowDQYJKoZIhvcNAQELBQAwJTEjMCEGA1UEAwwaYWN0aW9ucy5zZWxmLXNpZ25lZC5naXRodWIwHhcNMjQwMTIzMTUyNTM2WhcNMzQwMTIwMTUyNTM2WjAlMSMwIQYDVQQDDBphY3Rpb25zLnNlbGYtc2lnbmVkLmdpdGh1YjCCASIwDQYJKoZIhvcNAQEBBQADggEPADCCAQoCggEBAOTGp5svs8LJN8BH7VzXShWXnOK0lhDVuI0xnr5bwHFPc924CwaIEFb6mC7bvW2lZtgd633uaJ2naG6vKaOVGpCdGLE4ohH11nUk+2CNknZL7/oTmDHGSmGeHRb7kjtb0Ng4BJMPzmTYmCNUudfDFhHDcZz1Obuu85GsABrC5ZlzWzspYFXwUSaxvII+rHK/rAbOC2gmt5IOSLmgh3taQfp0mB6Lxlf89HoBPNwtPfBX8DtXTWQVnqODm4W+WfmWBSyXGX54DGNMyZwlTZqR0FjoMXxopId3MIuDGKxa2weDU5cW60N2y/qxikeV99fL3sg5aPA8s9iljKG0+MAfVNUCAwEAAaNTMFEwHQYDVR0OBBYEFIPALo5VanJ6E1B9eLQgGO+uGV65MB8GA1UdIwQYMBaAFIPALo5VanJ6E1B9eLQgGO+uGV65MA8GA1UdEwEB/wQFMAMBAf8wDQYJKoZIhvcNAQELBQADggEBAGS0hZE+DqKIRi49Z2KDOMOaSZnAYgqq6ws9HJHT09MXWlMHB8E/apvy2ZuFrcSu14ZLweJid+PrrooXEXEO6azEakzCjeUb9G1QwlzP4CkTcMGCw1Snh3jWZIuKaw21f7mp2rQ+YNltgHVDKY2s8AD273E8musEsWxJl80/MNvMie8Hfh4n4/Xl2r6t1YPmUJMoXAXdTBb0hkPy1fUu3r2T+1oi7Rw6kuVDfAZjaHupNHzJeDOg2KxUoK/GF2/M2qpVrd19Pv/JXNkQXRE4DFbErMmA7tXpp1tkXJRPhFui/Pv5H9cPgObEf9x6W4KnCXzT3ReeeRDKF8SqGTPELsc="],
            "x5t": "ykNaY4qM_ta4k2TgZOCEYLkcYlA"
        }, {
            "kty": "RSA",
            "alg": "RS256",
            "use": "sig",
            "kid": "1F2AB83404C08EC9EA0BB99DAED02186B091DBF4",
            "n": "u8zSYn5JR_O5yywSeOhmWWd7OMoLblh4iGTeIhTOVon-5e54RK30YQDeUCjpb9u3vdHTO7XS7i6EzkwLbsUOir27uhqoFGGWXSAZrPocOobSFoLC5l0NvSKRqVtpoADOHcAh59vLbr8dz3xtEEGx_qlLTzfFfWiCIYWiy15C2oo1eNPxzQfOvdu7Yet6Of4musV0Es5_mNETpeHOVEri8PWfxzw485UHIj3socl4Lk_I3iDyHfgpT49tIJYhHE5NImLNdwMha1cBCIbJMy1dJCfdoK827Hi9qKyBmftNQPhezGVRsOjsf2BfUGzGP5pCGrFBjEOcLhj_3j-TJebgvQ",
            "e": "AQAB",
            "x5c": ["MIIDrDCCApSgAwIBAgIQAP4blP36Q3WmMOhWf0RBMzANBgkqhkiG9w0BAQsFADA2MTQwMgYDVQQDEyt2c3RzLXZzdHNnaHJ0LWdoLXZzby1vYXV0aC52aXN1YWxzdHVkaW8uY29tMB4XDTIzMTAyNDE0NTI1NVoXDTI1MTAyNDE1MDI1NVowNjE0MDIGA1UEAxMrdnN0cy12c3RzZ2hydC1naC12c28tb2F1dGgudmlzdWFsc3R1ZGlvLmNvbTCCASIwDQYJKoZIhvcNAQEBBQADggEPADCCAQoCggEBALvM0mJ+SUfzucssEnjoZllnezjKC25YeIhk3iIUzlaJ/uXueESt9GEA3lAo6W/bt73R0zu10u4uhM5MC27FDoq9u7oaqBRhll0gGaz6HDqG0haCwuZdDb0ikalbaaAAzh3AIefby26/Hc98bRBBsf6pS083xX1ogiGFosteQtqKNXjT8c0Hzr3bu2Hrejn+JrrFdBLOf5jRE6XhzlRK4vD1n8c8OPOVByI97KHJeC5PyN4g8h34KU+PbSCWIRxOTSJizXcDIWtXAQiGyTMtXSQn3aCvNux4vaisgZn7TUD4XsxlUbDo7H9gX1Bsxj+aQhqxQYxDnC4Y/94/kyXm4L0CAwEAAaOBtTCBsjAOBgNVHQ8BAf8EBAMCBaAwCQYDVR0TBAIwADAdBgNVHSUEFjAUBggrBgEFBQcDAQYIKwYBBQUHAwIwNgYDVR0RBC8wLYIrdnN0cy12c3RzZ2hydC1naC12c28tb2F1dGgudmlzdWFsc3R1ZGlvLmNvbTAfBgNVHSMEGDAWgBSmWMP5CXuaSzoLKwcLXYZnoeCJmDAdBgNVHQ4EFgQUpljD+Ql7mks6CysHC12GZ6HgiZgwDQYJKoZIhvcNAQELBQADggEBAINwybFwYpXJkvauL5QbtrykIDYeP8oFdVIeVY8YI9MGfx7OwWDsNBVXv2B62zAZ49hK5G87++NmFI/FHnGOCISDYoJkRSCy2Nbeyr7Nx2VykWzUQqHLZfvr5KqW4Gj1OFHUqTl8lP3FWDd/P+lil3JobaSiICQshgF0GnX2a8ji8mfXpJSP20gzrLw84brmtmheAvJ9X/sLbM/RBkkT6g4NV2QbTMqo6k601qBNQBsH+lTDDWPCkRoAlW6a0z9bWIhGHWJ2lcR70zagcxIVl5/Fq35770/aMGroSrIx3JayOEqsvgIthYBKHzpT2VFwUz1VpBpNVJg9/u6jCwLY7QA="],
            "x5t": "Hyq4NATAjsnqC7mdrtAhhrCR2_Q"
        }, {
            "kty": "RSA",
            "alg": "RS256",
            "use": "sig",
            "kid": "001DDCD014A848E8824577B3E4F3AEDB3BCF5FFD",
            "n": "sI_r4iOwvRxksSovyZN8da5u-dh07fdcqh7FjyKKZCOVr7da898xk0TG9eZ7lfA1CmBTH4sX5evg4Yg2xdFDxYK4xmLZcwMyQZIDiZcdIujnttaqplrMv_v-YyAapHFmudbBO8NVuOH3gmGaJ02G8u1Vdf8C3PdNK13ch4wpNvyoxwqaIWGPSzudA6mGPGovRLhu5dEOOJSJtsLzExNvNmHnhPJZk06r7FePkBWSQ1CCHXAzpB-aUWEZC1FKMSiq2dvfOCyiJttEdyj8O_5yqb0wLAPb-8NdzkppbRal2WGowoU-AejqoWImhfDzlOBQStnhuAluKpA6sH0ifKlQsQ",
            "e": "AQAB",
            "x5c": ["MIIDrDCCApSgAwIBAgIQKiyRrA01T5qtxdzvZ/ErzjANBgkqhkiG9w0BAQsFADA2MTQwMgYDVQQDEyt2c3RzLXZzdHNnaHJ0LWdoLXZzby1vYXV0aC52aXN1YWxzdHVkaW8uY29tMB4XDTIzMTAxODE1MDExOFoXDTI1MTAxODE1MTExOFowNjE0MDIGA1UEAxMrdnN0cy12c3RzZ2hydC1naC12c28tb2F1dGgudmlzdWFsc3R1ZGlvLmNvbTCCASIwDQYJKoZIhvcNAQEBBQADggEPADCCAQoCggEBALCP6+IjsL0cZLEqL8mTfHWubvnYdO33XKoexY8iimQjla+3WvPfMZNExvXme5XwNQpgUx+LF+Xr4OGINsXRQ8WCuMZi2XMDMkGSA4mXHSLo57bWqqZazL/7/mMgGqRxZrnWwTvDVbjh94JhmidNhvLtVXX/Atz3TStd3IeMKTb8qMcKmiFhj0s7nQOphjxqL0S4buXRDjiUibbC8xMTbzZh54TyWZNOq+xXj5AVkkNQgh1wM6QfmlFhGQtRSjEoqtnb3zgsoibbRHco/Dv+cqm9MCwD2/vDXc5KaW0WpdlhqMKFPgHo6qFiJoXw85TgUErZ4bgJbiqQOrB9InypULECAwEAAaOBtTCBsjAOBgNVHQ8BAf8EBAMCBaAwCQYDVR0TBAIwADAdBgNVHSUEFjAUBggrBgEFBQcDAQYIKwYBBQUHAwIwNgYDVR0RBC8wLYIrdnN0cy12c3RzZ2hydC1naC12c28tb2F1dGgudmlzdWFsc3R1ZGlvLmNvbTAfBgNVHSMEGDAWgBQ45rBfvl4JJ7vg3WgLjQTfhDihvzAdBgNVHQ4EFgQUOOawX75eCSe74N1oC40E34Q4ob8wDQYJKoZIhvcNAQELBQADggEBABdN6HPheRdzwvJgi4xGHnf9pvlUC8981kAtgHnPT0VEYXh/dCMnKJSvCDJADpdmkuKxLxAfACeZR2CUHkQ0eO1ek/ihLvPqywDhLENq6Lvzu3qlhvUPBkGYjydpLtXQ1bBXUQ1FzT5/L1U19P2rJso9mC4ltu2OHJ9NLCKG0zffBItAJqhAiXtKbCUg4c9RbQxi9T2/xr9R72di4Qygfnmr3QleAqmjRG918cm5/uJ0s5EaK3QI7GQy7+tc44o3H3AI5eFtrHwIV0zoY4A9YIsaRmMHq9soHFBEO1HDKKRUOl/4tjpx8zHpp5Clz0wiZMgvSIdBa3/fTeUJ3flUYMo="],
            "x5t": "AB3c0BSoSOiCRXez5POu2zvPX_0"
        }]
    }"#;

    fn make_github_openid_config(issuer_uri: &str) -> String {
        format!(
            r#"{{
                "issuer": "{issuer_uri}",
                "jwks_uri": "{issuer_uri}/.well-known/jwks",
                "subject_types_supported": [
                    "public",
                    "pairwise"
                ],
                "response_types_supported": [
                    "id_token"
                ],
                "claims_supported": [
                    "sub",
                    "aud",
                    "exp",
                    "iat",
                    "iss",
                    "jti",
                    "nbf",
                    "ref",
                    "sha",
                    "repository",
                    "repository_id",
                    "repository_owner",
                    "repository_owner_id",
                    "enterprise",
                    "enterprise_id",
                    "run_id",
                    "run_number",
                    "run_attempt",
                    "actor",
                    "actor_id",
                    "workflow",
                    "workflow_ref",
                    "workflow_sha",
                    "head_ref",
                    "base_ref",
                    "event_name",
                    "ref_type",
                    "ref_protected",
                    "environment",
                    "environment_node_id",
                    "job_workflow_ref",
                    "job_workflow_sha",
                    "repository_visibility",
                    "runner_environment",
                    "issuer_scope"
                ],
                "id_token_signing_alg_values_supported": [
                    "RS256"
                ],
                "scopes_supported": [
                    "openid"
                ]
            }}"#,
        )
    }

    #[tokio::test]
    async fn test_load_jwks() {
        let mut server = mockito::Server::new_async().await;

        let issuer_url = server.url();

        let _config_mock = server
            .mock("GET", "/.well-known/openid-configuration")
            .with_header("content-type", "application/json")
            .with_body(make_github_openid_config(&issuer_url))
            .create();

        let _jwks_mock = server
            .mock("GET", "/.well-known/jwks")
            .with_header("content-type", "application/json")
            .with_body(GITHUB_JWKS)
            .create();

        let client = Client::new();
        let jwks = assert_ok!(load_jwks(&client, &issuer_url).await);
        assert_debug_snapshot!(jwks);
    }
}
