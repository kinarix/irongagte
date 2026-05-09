pub mod magic_link;
pub mod password;
pub mod session;
pub mod totp;

pub use magic_link::MagicLinkService;
pub use password::PasswordService;
pub use session::SessionService;
pub use totp::TotpService;
