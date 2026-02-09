//! Android JNI 工具函数
//! 
//! 提供 JNI 操作的辅助函数和工具

#[cfg(feature = "android")]
use jni::JNIEnv;
#[cfg(feature = "android")]
use jni::objects::{JClass, JObject, JString, JValue};
#[cfg(feature = "android")]
use jni::sys::{jobject, jobjectArray, jstring};
#[cfg(feature = "android")]
use std::ffi::{CStr, CString};
#[cfg(feature = "android")]
use std::os::raw::c_char;

/// 将 Java 字符串转换为 Rust 字符串
#[cfg(feature = "android")]
pub fn java_string_to_rust_string(env: &JNIEnv, java_string: JString) -> Result<String, Box<dyn std::error::Error>> {
    let rust_string = env.get_string(java_string)?;
    Ok(rust_string.to_string())
}

/// 将 Rust 字符串转换为 Java 字符串
#[cfg(feature = "android")]
pub fn rust_string_to_java_string(env: &JNIEnv, rust_string: &str) -> Result<jstring, Box<dyn std::error::Error>> {
    let java_string = env.new_string(rust_string)?;
    Ok(java_string.into_raw())
}

/// 将 C 字符串转换为 Rust 字符串（安全版本）
#[cfg(feature = "android")]
pub unsafe fn c_string_to_rust_string(c_str: *const c_char) -> Result<String, Box<dyn std::error::Error>> {
    if c_str.is_null() {
        return Err("C 字符串指针为空".into());
    }
    
    let rust_str = CStr::from_ptr(c_str).to_str()?;
    Ok(rust_str.to_string())
}

/// 将 Rust 字符串转换为 C 字符串
#[cfg(feature = "android")]
pub fn rust_string_to_c_string(rust_string: &str) -> Result<CString, Box<dyn std::error::Error>> {
    let c_string = CString::new(rust_string)?;
    Ok(c_string)
}

/// 检查 Java 对象是否为空
#[cfg(feature = "android")]
pub fn is_null_object(obj: JObject) -> bool {
    obj.is_null()
}

/// 获取 Java 类的简单名称
#[cfg(feature = "android")]
pub fn get_class_simple_name(env: &JNIEnv, class: JClass) -> Result<String, Box<dyn std::error::Error>> {
    let class_obj = class.into();
    let name_method_id = env.get_method_id(class_obj, "getSimpleName", "()Ljava/lang/String;")?;
    let name_obj = env.call_method_unchecked(
        class_obj,
        name_method_id,
        jni::signature::ReturnType::Object,
        &[]
    )?;
    
    if let JValue::Object(name_string) = name_obj {
        let name_jstring = JString::from(name_string);
        java_string_to_rust_string(env, name_jstring)
    } else {
        Err("无法获取类名".into())
    }
}

/// 创建 Java 字符串数组
#[cfg(feature = "android")]
pub fn create_java_string_array(env: &JNIEnv, strings: &[String]) -> Result<jobjectArray, Box<dyn std::error::Error>> {
    let string_class = env.find_class("java/lang/String")?;
    let array = env.new_object_array(strings.len() as jni::sys::jsize, string_class, JObject::null())?;
    
    for (i, string) in strings.iter().enumerate() {
        let java_string = env.new_string(string)?;
        env.set_object_array_element(array, i as jni::sys::jsize, java_string)?;
    }
    
    Ok(array.into_raw())
}

/// 从 Java 字符串数组转换为 Rust Vec<String>
#[cfg(feature = "android")]
pub fn java_string_array_to_rust_vec(env: &JNIEnv, array: jobjectArray) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    if array.is_null() {
        return Ok(Vec::new());
    }
    
    let length = env.get_array_length(array)?;
    let mut result = Vec::with_capacity(length as usize);
    
    for i in 0..length {
        let element = env.get_object_array_element(array, i)?;
        let string = JString::from(element);
        let rust_string = java_string_to_rust_string(env, string)?;
        result.push(rust_string);
    }
    
    Ok(result)
}

/// JNI 错误处理
#[cfg(feature = "android)]
#[derive(Debug)]
pub enum JniError {
    NullPointer,
    TypeMismatch,
    MethodNotFound,
    FieldNotFound,
    ExceptionOccurred,
    OutOfMemory,
    InvalidArgument,
    Unknown(String),
}

impl std::fmt::Display for JniError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JniError::NullPointer => write!(f, "JNI 空指针错误"),
            JniError::TypeMismatch => write!(f, "JNI 类型不匹配"),
            JniError::MethodNotFound => write!(f, "JNI 方法未找到"),
            JniError::FieldNotFound => write!(f, "JNI 字段未找到"),
            JniError::ExceptionOccurred => write!(f, "JNI 异常发生"),
            JniError::OutOfMemory => write!(f, "JNI 内存不足"),
            JniError::InvalidArgument => write!(f, "JNI 无效参数"),
            JniError::Unknown(msg) => write!(f, "JNI 未知错误: {}", msg),
        }
    }
}

impl std::error::Error for JniError {}

/// 检查 JNI 异常并记录
#[cfg(feature = "android")]
pub fn check_and_log_jni_exception(env: &JNIEnv, context: &str) -> Result<(), JniError> {
    if env.exception_check().unwrap_or(false) {
        let exception = env.exception_occurred().unwrap_or(JObject::null());
        let exception_info = if exception.is_null() {
            "未知异常".to_string()
        } else {
            // 尝试获取异常信息
            format!("异常对象: {:?}", exception)
        };
        
        log::error!("JNI 异常在 {}: {}", context, exception_info);
        let _ = env.exception_clear();
        Err(JniError::ExceptionOccurred)
    } else {
        Ok(())
    }
}

/// 安全的 JNI 调用包装器
#[cfg(feature = "android)]
pub fn safe_jni_call<F, R>(env: &JNIEnv, context: &str, func: F) -> Result<R, JniError>
where
    F: FnOnce() -> Result<R, jni::errors::Error>,
{
    match func() {
        Ok(result) => {
            check_and_log_jni_exception(env, context)?;
            Ok(result)
        }
        Err(jni_error) => {
            log::error!("JNI 调用失败在 {}: {:?}", context, jni_error);
            Err(JniError::Unknown(jni_error.to_string()))
        }
    }
}
