fn main() {
    #[cfg(target_os = "macos")]
    {
        println!("cargo:rustc-link-lib=framework=AppKit");
    }
    #[cfg(target_os = "windows")]
    {
        embed_resource::compile("app_icon.rc");
    }
}