fn main() {
    #[cfg(target_os = "macos")]
    {
        println!("cargo:rustc-link-lib=framework=AppKit");
    }
    
    #[cfg(target_os = "windows")]
    {
        // 尝试编译图标资源文件
        if std::path::Path::new("assets/icon/app_icon.ico").exists() {
            match embed_resource::compile("app_icon.rc") {
                Ok(_) => println!("✅ Windows图标资源编译成功"),
                Err(e) => {
                    println!("cargo:warning=图标资源编译失败: {}", e);
                    println!("cargo:warning=应用将使用默认图标");
                    // 不要让构建失败，只是警告
                }
            }
        } else {
            println!("cargo:warning=未找到app_icon.ico文件，跳过图标资源编译");
        }
    }
}