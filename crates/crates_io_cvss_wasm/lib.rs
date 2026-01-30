//! WASM module for CVSS (Common Vulnerability Scoring System) score calculation.
//!
//! Provides functions to parse CVSS vector strings (v3.0, v3.1, v4.0)
//! and calculate their scores and severity ratings.

use std::{convert::Infallible, str::FromStr};

use cvss::Cvss;
use wasm_bindgen::prelude::*;

/// Parse a CVSS vector string and calculate its score.
///
/// Supports CVSS v3.0, v3.1, and v4.0 vector strings.
///
/// # Arguments
/// * `vector` - A CVSS vector string (e.g., "CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:U/C:H/I:H/A:H")
///
/// # Returns
/// A JavaScript object with score, severity, version, valid flag, and optional error.
#[wasm_bindgen]
pub fn parse_cvss(vector: &str) -> JsValue {
    let result = CvssResult::from_str(vector).unwrap();
    serde_wasm_bindgen::to_value(&result).unwrap_or(JsValue::NULL)
}

/// Result of parsing and scoring a CVSS vector.
#[derive(serde::Serialize)]
pub struct CvssResult {
    /// The calculated CVSS score (0.0 - 10.0)
    pub score: f64,
    /// The severity rating (None, Low, Medium, High, Critical)
    pub severity: String,
    /// The CVSS version (e.g., "3.0", "3.1", "4.0")
    pub version: String,
    /// Whether parsing was successful
    pub valid: bool,
    /// Error message if parsing failed
    pub error: Option<String>,
}

impl FromStr for CvssResult {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match Cvss::from_str(s) {
            Ok(cvss) => Ok(Self {
                score: cvss.score(),
                severity: cvss.severity().to_string(),
                version: match cvss {
                    Cvss::CvssV30(_) => "3.0".to_string(),
                    Cvss::CvssV31(_) => "3.1".to_string(),
                    Cvss::CvssV40(_) => "4.0".to_string(),
                    _ => "Unknown".to_string(),
                },
                valid: true,
                error: None,
            }),
            Err(error) => Ok(CvssResult {
                score: 0.0,
                severity: "Unknown".to_string(),
                version: "Unknown".to_string(),
                valid: false,
                error: Some(error.to_string()),
            }),
        }
    }
}

/// Initialize the WASM module with better panic messages.
#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cvss_v31_critical() {
        let vector = "CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:U/C:H/I:H/A:H";
        let result = CvssResult::from_str(vector).unwrap();
        assert!(result.valid);
        assert_eq!(result.score, 9.8);
        assert_eq!(result.severity, "critical");
        assert_eq!(result.version, "3.1");
    }

    #[test]
    fn test_cvss_v30() {
        let vector = "CVSS:3.0/AV:N/AC:L/PR:N/UI:N/S:U/C:H/I:H/A:H";
        let result = CvssResult::from_str(vector).unwrap();
        assert!(result.valid);
        assert_eq!(result.score, 9.8);
        assert_eq!(result.version, "3.0");
    }

    #[test]
    fn test_cvss_v4() {
        let vector = "CVSS:4.0/AV:N/AC:L/AT:N/PR:N/UI:N/VC:H/VI:H/VA:H/SC:N/SI:N/SA:N";
        let result = CvssResult::from_str(vector).unwrap();
        assert!(result.valid);
        assert_eq!(result.version, "4.0");
        assert!(result.score > 0.0);
    }

    #[test]
    fn test_invalid_vector() {
        let vector = "invalid";
        let result = CvssResult::from_str(vector).unwrap();
        assert!(!result.valid);
        assert!(result.error.is_some());
    }
}
