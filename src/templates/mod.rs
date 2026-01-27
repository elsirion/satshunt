pub mod admin_locations;
pub mod admin_users;
pub mod collect;
pub mod components;
pub mod donate;
pub mod home;
pub mod layout;
pub mod location_detail;
pub mod login;
pub mod map;
pub mod new_location;
pub mod profile;
pub mod register;
pub mod wallet;
pub mod withdraw;

/// Format sats with SI prefixes (k, M) and 3 significant figures
pub fn format_sats_si(sats: i64) -> String {
    if sats <= 0 {
        return "0".to_string();
    }
    let sats = sats as f64;

    let (val, suffix) = if sats >= 1_000_000.0 {
        (sats / 1_000_000.0, "M")
    } else if sats >= 1_000.0 {
        (sats / 1_000.0, "k")
    } else {
        return (sats as u64).to_string();
    };

    // Round to 3 significant figures
    let decimals = if val >= 100.0 { 0 } else if val >= 10.0 { 1 } else { 2 };
    let formatted = format!("{:.decimals$}{suffix}", val);

    // Handle rounding up to next unit (e.g., 999.9k -> 1000k should be 1M)
    if formatted.starts_with("1000") && suffix == "k" {
        "1.00M".to_string()
    } else {
        formatted
    }
}

pub use admin_locations::admin_locations;
pub use admin_users::admin_users;
pub use collect::{collect, CollectParams};
pub use donate::donate;
pub use home::home;
pub use layout::{base, base_with_user};
pub use location_detail::location_detail;
pub use login::login;
pub use map::map;
pub use new_location::new_location;
pub use profile::profile;
pub use register::register;
pub use wallet::wallet;
pub use withdraw::withdraw;
