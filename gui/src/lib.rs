pub mod app;
pub mod client;
pub mod lifecycle;
pub mod model;
pub mod preferences;
pub mod presentation;
pub mod protocol;
pub mod statistics;
pub mod tray;
pub mod ui {
    slint::include_modules!();
}
