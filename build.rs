fn main() {
    #[cfg(target_os = "macos")]
    {
        println!("cargo:rustc-link-lib=framework=AppKit");
    }
    
    #[cfg(target_os = "windows")]
    {
        // 检查图标文件是否存在，然后编译资源
        if std::path::Path::new("assets/icon/app_icon.ico").exists() && 
           std::path::Path::new("app_icon.rc").exists() {
            embed_resource::compile("app_icon.rc");
        }
    }
}