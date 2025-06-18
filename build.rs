use std::env;

fn main() {
    // 只在Windows平台编译时处理资源文件
    if env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() == "windows" {
        // 如果图标文件发生变化，重新运行构建脚本
        println!("cargo:rerun-if-changed=Icon.png");
        println!("cargo:rerun-if-changed=resources.rc");
        
        // 编译并链接资源文件
        embed_resource::compile("resources.rc", embed_resource::NONE);
    }
}