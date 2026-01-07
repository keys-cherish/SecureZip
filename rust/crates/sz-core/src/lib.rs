//! SecureZip 核心模块
//! 
//! 提供公共类型定义和错误处理

pub mod error;
pub mod types;

pub use error::{SzError, SzResult};
pub use types::*;
