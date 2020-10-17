//! Endpoints that provide badges based on crate metadata

use crate::controllers::frontend_prelude::*;

use crate::models::{Badge, Crate, CrateBadge, MaintenanceStatus};
use crate::schema::*;

use conduit::{Body, Response};

/// Handles the `GET /crates/:crate_id/maintenance.svg` route.
pub fn maintenance(req: &mut dyn RequestExt) -> EndpointResult {
    let name = &req.params()["crate_id"];
    let conn = req.db_read_only()?;

    let krate = Crate::by_name(name).first::<Crate>(&*conn);
    if krate.is_err() {
        let response = Response::builder().status(404).body(Body::empty()).unwrap();

        return Ok(response);
    }

    let krate = krate.unwrap();

    let maintenance_badge: Option<CrateBadge> = CrateBadge::belonging_to(&krate)
        .select((badges::crate_id, badges::all_columns))
        .load::<CrateBadge>(&*conn)?
        .into_iter()
        .find(|cb| matches!(cb.badge, Badge::Maintenance { .. }));

    let status = maintenance_badge
        .map(|it| match it.badge {
            Badge::Maintenance { status } => Some(status),
            _ => None,
        })
        .flatten();

    let badge = generate_badge(status);

    let response = Response::builder()
        .status(200)
        .body(Body::from_vec(badge.into_bytes()))
        .unwrap();

    Ok(response)
}

fn generate_badge(status: Option<MaintenanceStatus>) -> String {
    let message = match status {
        Some(MaintenanceStatus::ActivelyDeveloped) => "actively-developed",
        Some(MaintenanceStatus::PassivelyMaintained) => "passively-maintained",
        Some(MaintenanceStatus::AsIs) => "as-is",
        Some(MaintenanceStatus::None) => "unknown",
        Some(MaintenanceStatus::Experimental) => "experimental",
        Some(MaintenanceStatus::LookingForMaintainer) => "looking-for-maintainer",
        Some(MaintenanceStatus::Deprecated) => "deprecated",
        None => "unknown",
    };

    let color = match status {
        Some(MaintenanceStatus::ActivelyDeveloped) => "brightgreen",
        Some(MaintenanceStatus::PassivelyMaintained) => "yellowgreen",
        Some(MaintenanceStatus::AsIs) => "yellow",
        Some(MaintenanceStatus::None) => "lightgrey",
        Some(MaintenanceStatus::Experimental) => "blue",
        Some(MaintenanceStatus::LookingForMaintainer) => "orange",
        Some(MaintenanceStatus::Deprecated) => "red",
        None => "lightgrey",
    };

    let badge_options = badge::BadgeOptions {
        subject: "maintenance".to_owned(),
        status: message.to_owned(),
        color: color.to_string(),
    };

    badge::Badge::new(badge_options).unwrap().to_svg()
}
