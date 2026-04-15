use dioxus::{
    desktop::{wry::dpi::Size, Config, WindowBuilder},
    prelude::*,
};

use components::Hero;

/// Define a components module that contains all shared components for our app.
mod components;
mod config;

// We can import assets in dioxus with the `asset!` macro. This macro takes a path to an asset relative to the crate root.
// The macro returns an `Asset` type that will display as the path to the asset in the browser or a local path in desktop bundles.
const FAVICON: Asset = asset!("/assets/favicon.ico");
// The asset macro also minifies some assets like CSS and JS to make bundled smaller
const MAIN_CSS: Asset = asset!("/assets/styling/main.css");
const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

fn main() {
    let window_builder = WindowBuilder::new()
        .with_title("lentil".to_string())
        .with_always_on_top(true)
        .with_resizable(false)
        .with_focused(true)
        .with_maximizable(false)
        .with_decorations(false)
        .with_inner_size(Size::Logical(dioxus::desktop::LogicalSize {
            width: 680.0,
            height: 560.0,
        }));
    let fullscreen_config = Config::new().with_window(window_builder);
    dioxus::LaunchBuilder::new()
        .with_cfg(fullscreen_config)
        .launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "stylesheet", href: MAIN_CSS }
        document::Link { rel: "stylesheet", href: TAILWIND_CSS }

        Hero {}

    }
}
