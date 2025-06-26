//! Module for number formatting functions.
//!
//! This module contains utility functions for formatting numbers in various ways,
//! such as human-readable byte sizes.

/// Formats a byte size value into a human-readable string.
///
/// The function follows these rules:
/// - Uses units: B, kB and MB
/// - Switches from B to kB at 1500 bytes
/// - Switches from kB to MB at 1500 * 1024 bytes
/// - Limits the number to a maximum of 4 characters by adjusting decimal places
///
/// # Arguments
///
/// * `bytes` - The size in bytes to format
///
/// # Returns
///
/// A formatted string representing the size with appropriate units
pub fn format_bytes(bytes: u32) -> String {
    const THRESHOLD: f64 = 1500.;
    const UNITS: &[&str] = &["B", "kB", "MB"];

    let mut value = bytes as f64;
    let mut unit_index = 0;

    // Keep dividing by 1024 until value is below threshold or we've reached the last unit
    while value >= THRESHOLD && unit_index < UNITS.len() - 1 {
        value /= 1024.0;
        unit_index += 1;
    }

    let unit = UNITS[unit_index];

    // Special case for bytes - no decimal places
    if unit_index == 0 {
        return format!("{bytes} {unit}");
    }

    // For kB and MB, format with appropriate decimal places

    // Determine number of decimal places to keep number under 4 chars
    if value < 10.0 {
        format!("{value:.2} {unit}") // e.g., 1.50 kB, 9.99 MB
    } else if value < 100.0 {
        format!("{value:.1} {unit}") // e.g., 10.5 kB, 99.9 MB
    } else {
        format!("{value:.0} {unit}") // e.g., 100 kB, 999 MB
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes() {
        // Test bytes format (below 1500 bytes)
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(1), "1 B");
        assert_eq!(format_bytes(1000), "1000 B");
        assert_eq!(format_bytes(1499), "1499 B");

        // Test kilobytes format (1500 bytes to 1500 * 1024 bytes)
        assert_eq!(format_bytes(1500), "1.46 kB");
        assert_eq!(format_bytes(2048), "2.00 kB");
        assert_eq!(format_bytes(5120), "5.00 kB");
        assert_eq!(format_bytes(10240), "10.0 kB");
        assert_eq!(format_bytes(51200), "50.0 kB");
        assert_eq!(format_bytes(102400), "100 kB");
        assert_eq!(format_bytes(512000), "500 kB");
        assert_eq!(format_bytes(1048575), "1024 kB");

        // Test megabytes format (above 1500 * 1024 bytes)
        assert_eq!(format_bytes(1536000), "1.46 MB");
        assert_eq!(format_bytes(2097152), "2.00 MB");
        assert_eq!(format_bytes(5242880), "5.00 MB");
        assert_eq!(format_bytes(10485760), "10.0 MB");
        assert_eq!(format_bytes(52428800), "50.0 MB");
        assert_eq!(format_bytes(104857600), "100 MB");
        assert_eq!(format_bytes(1073741824), "1024 MB");
    }
}
