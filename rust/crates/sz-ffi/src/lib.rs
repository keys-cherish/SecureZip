//! SecureZip FFI 模块
//! 
//! 为 Flutter 提供 Rust 功能的 FFI 接口
//! 使用纯 C ABI，可被 dart:ffi 直接调用

pub mod api;
pub mod c_api;
// pub mod frb_generated;  // 使用手动 C ABI，不需要 FRB 生成的代码

pub use api::*;
pub use c_api::*;
