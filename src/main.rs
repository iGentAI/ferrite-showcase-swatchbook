#![forbid(unsafe_code)]
//! Process entry point. The Ferrite harness reads the injected context, builds
//! the async runtime and runs [`swatchbook::App`].

fn main() {
    ferrite_app::app_main::<swatchbook::App>()
}
