pub mod layout;
pub mod home;
pub mod map;
pub mod new_location;
pub mod location_detail;
pub mod nfc_setup;
pub mod donate;
pub mod login;
pub mod register;

pub use layout::{base, base_with_user};
pub use home::home;
pub use map::map;
pub use new_location::new_location;
pub use location_detail::location_detail;
pub use nfc_setup::nfc_setup;
pub use donate::donate;
pub use login::login;
pub use register::register;
