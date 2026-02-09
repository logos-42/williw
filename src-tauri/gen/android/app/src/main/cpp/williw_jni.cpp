#include <jni.h>
#include <string>
#include <android/log.h>
#include <memory>

// Rust FFI头文件
extern "C" {
    // Rust FFI函数声明
    jlong Java_com_williw_mobile_WilliwNode_createNode(JNIEnv* env, jobject thiz);
    void Java_com_williw_mobile_WilliwNode_destroyNode(JNIEnv* env, jobject thiz, jlong ptr);
    jstring Java_com_williw_mobile_WilliwNode_getCapabilities(JNIEnv* env, jobject thiz, jlong ptr);
    jint Java_com_williw_mobile_WilliwNode_updateNetworkType(JNIEnv* env, jobject thiz, jlong ptr, jstring network_type);
    jint Java_com_williw_mobile_WilliwNode_updateBattery(JNIEnv* env, jobject thiz, jlong ptr, jfloat level, jint is_charging);
    jint Java_com_williw_mobile_WilliwNode_refreshDeviceInfo(JNIEnv* env, jobject thiz, jlong ptr);
    jint Java_com_williw_mobile_WilliwNode_recommendedModelDim(JNIEnv* env, jobject thiz, jlong ptr);
    jlong Java_com_williw_mobile_WilliwNode_recommendedTickInterval(JNIEnv* env, jobject thiz, jlong ptr);
    jint Java_com_williw_mobile_WilliwNode_shouldPauseTraining(JNIEnv* env, jobject thiz, jlong ptr);
}

// 日志标签
#define LOG_TAG "WilliwJNI"
#define LOGI(...) __android_log_print(ANDROID_LOG_INFO, LOG_TAG, __VA_ARGS__)
#define LOGE(...) __android_log_print(ANDROID_LOG_ERROR, LOG_TAG, __VA_ARGS__)
#define LOGD(...) __android_log_print(ANDROID_LOG_DEBUG, LOG_TAG, __VA_ARGS__)

// Rust FFI函数指针类型
typedef jlong (*CreateNodeFunc)(JNIEnv*, jobject);
typedef void (*DestroyNodeFunc)(JNIEnv*, jobject, jlong);
typedef jstring (*GetCapabilitiesFunc)(JNIEnv*, jobject, jlong);
typedef jint (*UpdateNetworkTypeFunc)(JNIEnv*, jobject, jlong, jstring);
typedef jint (*UpdateBatteryFunc)(JNIEnv*, jobject, jlong, jfloat, jint);
typedef jint (*RefreshDeviceInfoFunc)(JNIEnv*, jobject, jlong);
typedef jint (*RecommendedModelDimFunc)(JNIEnv*, jobject, jlong);
typedef jlong (*RecommendedTickIntervalFunc)(JNIEnv*, jobject, jlong);
typedef jint (*ShouldPauseTrainingFunc)(JNIEnv*, jobject, jlong);

// Rust库句柄
void* rust_lib_handle = nullptr;

// Rust函数指针
CreateNodeFunc rust_create_node = nullptr;
DestroyNodeFunc rust_destroy_node = nullptr;
GetCapabilitiesFunc rust_get_capabilities = nullptr;
UpdateNetworkTypeFunc rust_update_network_type = nullptr;
UpdateBatteryFunc rust_update_battery = nullptr;
RefreshDeviceInfoFunc rust_refresh_device_info = nullptr;
RecommendedModelDimFunc rust_recommended_model_dim = nullptr;
RecommendedTickIntervalFunc rust_recommended_tick_interval = nullptr;
ShouldPauseTrainingFunc rust_should_pause_training = nullptr;

/**
 * 加载Rust库
 */
bool loadRustLibrary() {
    if (rust_lib_handle != nullptr) {
        return true; // 已经加载
    }
    
    // 尝试加载Rust库
    rust_lib_handle = dlopen("libwilliw.so", RTLD_LAZY);
    if (rust_lib_handle == nullptr) {
        LOGE("无法加载libwilliw.so: %s", dlerror());
        return false;
    }
    
    // 获取函数指针
    rust_create_node = (CreateNodeFunc) dlsym(rust_lib_handle, "williw_node_create");
    rust_destroy_node = (DestroyNodeFunc) dlsym(rust_lib_handle, "williw_node_destroy");
    rust_get_capabilities = (GetCapabilitiesFunc) dlsym(rust_lib_handle, "williw_node_get_capabilities");
    rust_update_network_type = (UpdateNetworkTypeFunc) dlsym(rust_lib_handle, "williw_node_update_network_type");
    rust_update_battery = (UpdateBatteryFunc) dlsym(rust_lib_handle, "williw_node_update_battery");
    rust_refresh_device_info = (RefreshDeviceInfoFunc) dlsym(rust_lib_handle, "williw_node_refresh_device_info");
    rust_recommended_model_dim = (RecommendedModelDimFunc) dlsym(rust_lib_handle, "williw_node_recommended_model_dim");
    rust_recommended_tick_interval = (RecommendedTickIntervalFunc) dlsym(rust_lib_handle, "williw_node_recommended_tick_interval");
    rust_should_pause_training = (ShouldPauseTrainingFunc) dlsym(rust_lib_handle, "williw_node_should_pause_training");
    
    // 检查所有函数指针是否有效
    if (!rust_create_node || !rust_destroy_node || !rust_get_capabilities || 
        !rust_update_network_type || !rust_update_battery || !rust_refresh_device_info ||
        !rust_recommended_model_dim || !rust_recommended_tick_interval || !rust_should_pause_training) {
        LOGE("无法获取所有Rust函数指针");
        dlclose(rust_lib_handle);
        rust_lib_handle = nullptr;
        return false;
    }
    
    LOGI("Rust库加载成功");
    return true;
}

/**
 * JNI_OnLoad - 在库加载时调用
 */
JNIEXPORT jint JNICALL JNI_OnLoad(JavaVM* vm, void* reserved) {
    LOGI("JNI_OnLoad called");
    
    JNIEnv* env;
    if (vm->GetEnv(reinterpret_cast<void**>(&env), JNI_VERSION_1_6) != JNI_OK) {
        LOGE("Failed to get JNIEnv");
        return JNI_ERR;
    }
    
    // 尝试加载Rust库
    if (!loadRustLibrary()) {
        LOGE("Failed to load Rust library");
        // 不返回错误，允许在没有Rust库的情况下运行
    }
    
    return JNI_VERSION_1_6;
}

/**
 * JNI_OnUnload - 在库卸载时调用
 */
JNIEXPORT void JNICALL JNI_OnUnload(JavaVM* vm, void* reserved) {
    LOGI("JNI_OnUnload called");
    
    if (rust_lib_handle != nullptr) {
        dlclose(rust_lib_handle);
        rust_lib_handle = nullptr;
    }
}

/**
 * 创建Williw节点
 */
extern "C" JNIEXPORT jlong JNICALL
Java_com_williw_mobile_WilliwNode_createNode(JNIEnv* env, jobject thiz) {
    LOGD("createNode called");
    
    if (!loadRustLibrary()) {
        LOGE("Rust库未加载，返回空指针");
        return 0;
    }
    
    if (rust_create_node == nullptr) {
        LOGE("rust_create_node函数指针为空");
        return 0;
    }
    
    jlong result = rust_create_node(env, thiz);
    LOGD("createNode result: %ld", (long)result);
    return result;
}

/**
 * 销毁Williw节点
 */
extern "C" JNIEXPORT void JNICALL
Java_com_williw_mobile_WilliwNode_destroyNode(JNIEnv* env, jobject thiz, jlong ptr) {
    LOGD("destroyNode called with ptr: %ld", (long)ptr);
    
    if (ptr == 0) {
        LOGE("destroyNode: ptr is null");
        return;
    }
    
    if (!loadRustLibrary() || rust_destroy_node == nullptr) {
        LOGE("Rust库未加载或函数指针为空");
        return;
    }
    
    rust_destroy_node(env, thiz, ptr);
    LOGD("destroyNode completed");
}

/**
 * 获取设备能力
 */
extern "C" JNIEXPORT jstring JNICALL
Java_com_williw_mobile_WilliwNode_getCapabilities(JNIEnv* env, jobject thiz, jlong ptr) {
    LOGD("getCapabilities called with ptr: %ld", (long)ptr);
    
    if (ptr == 0) {
        LOGE("getCapabilities: ptr is null");
        return env->NewStringUTF("{\"error\":\"Node pointer is null\"}");
    }
    
    if (!loadRustLibrary() || rust_get_capabilities == nullptr) {
        LOGE("Rust库未加载或函数指针为空");
        return env->NewStringUTF("{\"error\":\"Rust library not loaded\"}");
    }
    
    jstring result = rust_get_capabilities(env, thiz, ptr);
    LOGD("getCapabilities completed");
    return result;
}

/**
 * 更新网络类型
 */
extern "C" JNIEXPORT jint JNICALL
Java_com_williw_mobile_WilliwNode_updateNetworkType(JNIEnv* env, jobject thiz, jlong ptr, jstring network_type) {
    LOGD("updateNetworkType called with ptr: %ld", (long)ptr);
    
    if (ptr == 0) {
        LOGE("updateNetworkType: ptr is null");
        return -1;
    }
    
    if (!loadRustLibrary() || rust_update_network_type == nullptr) {
        LOGE("Rust库未加载或函数指针为空");
        return -2;
    }
    
    jint result = rust_update_network_type(env, thiz, ptr, network_type);
    LOGD("updateNetworkType result: %d", result);
    return result;
}

/**
 * 更新电池状态
 */
extern "C" JNIEXPORT jint JNICALL
Java_com_williw_mobile_WilliwNode_updateBattery(JNIEnv* env, jobject thiz, jlong ptr, jfloat level, jint is_charging) {
    LOGD("updateBattery called with ptr: %ld, level: %f, is_charging: %d", (long)ptr, level, is_charging);
    
    if (ptr == 0) {
        LOGE("updateBattery: ptr is null");
        return -1;
    }
    
    if (!loadRustLibrary() || rust_update_battery == nullptr) {
        LOGE("Rust库未加载或函数指针为空");
        return -2;
    }
    
    jint result = rust_update_battery(env, thiz, ptr, level, is_charging);
    LOGD("updateBattery result: %d", result);
    return result;
}

/**
 * 刷新设备信息
 */
extern "C" JNIEXPORT jint JNICALL
Java_com_williw_mobile_WilliwNode_refreshDeviceInfo(JNIEnv* env, jobject thiz, jlong ptr) {
    LOGD("refreshDeviceInfo called with ptr: %ld", (long)ptr);
    
    if (ptr == 0) {
        LOGE("refreshDeviceInfo: ptr is null");
        return -1;
    }
    
    if (!loadRustLibrary() || rust_refresh_device_info == nullptr) {
        LOGE("Rust库未加载或函数指针为空");
        return -2;
    }
    
    jint result = rust_refresh_device_info(env, thiz, ptr);
    LOGD("refreshDeviceInfo result: %d", result);
    return result;
}

/**
 * 获取推荐模型维度
 */
extern "C" JNIEXPORT jint JNICALL
Java_com_williw_mobile_WilliwNode_recommendedModelDim(JNIEnv* env, jobject thiz, jlong ptr) {
    LOGD("recommendedModelDim called with ptr: %ld", (long)ptr);
    
    if (ptr == 0) {
        LOGE("recommendedModelDim: ptr is null");
        return 256; // 默认值
    }
    
    if (!loadRustLibrary() || rust_recommended_model_dim == nullptr) {
        LOGE("Rust库未加载或函数指针为空");
        return 256; // 默认值
    }
    
    jint result = rust_recommended_model_dim(env, thiz, ptr);
    LOGD("recommendedModelDim result: %d", result);
    return result;
}

/**
 * 获取推荐训练间隔
 */
extern "C" JNIEXPORT jlong JNICALL
Java_com_williw_mobile_WilliwNode_recommendedTickInterval(JNIEnv* env, jobject thiz, jlong ptr) {
    LOGD("recommendedTickInterval called with ptr: %ld", (long)ptr);
    
    if (ptr == 0) {
        LOGE("recommendedTickInterval: ptr is null");
        return 10; // 默认值
    }
    
    if (!loadRustLibrary() || rust_recommended_tick_interval == nullptr) {
        LOGE("Rust库未加载或函数指针为空");
        return 10; // 默认值
    }
    
    jlong result = rust_recommended_tick_interval(env, thiz, ptr);
    LOGD("recommendedTickInterval result: %ld", (long)result);
    return result;
}

/**
 * 检查是否应该暂停训练
 */
extern "C" JNIEXPORT jint JNICALL
Java_com_williw_mobile_WilliwNode_shouldPauseTraining(JNIEnv* env, jobject thiz, jlong ptr) {
    LOGD("shouldPauseTraining called with ptr: %ld", (long)ptr);
    
    if (ptr == 0) {
        LOGE("shouldPauseTraining: ptr is null");
        return 0; // 默认不暂停
    }
    
    if (!loadRustLibrary() || rust_should_pause_training == nullptr) {
        LOGE("Rust库未加载或函数指针为空");
        return 0; // 默认不暂停
    }
    
    jint result = rust_should_pause_training(env, thiz, ptr);
    LOGD("shouldPauseTraining result: %d", result);
    return result;
}

/**
 * 获取JNI版本
 */
extern "C" JNIEXPORT jstring JNICALL
Java_com_williw_mobile_WilliwNode_getJniVersion(JNIEnv* env, jobject thiz) {
    std::string version = "JNI_VERSION_1_6";
    return env->NewStringUTF(version.c_str());
}

/**
 * 检查Rust库是否已加载
 */
extern "C" JNIEXPORT jboolean JNICALL
Java_com_williw_mobile_WilliwNode_isRustLibraryLoaded(JNIEnv* env, jobject thiz) {
    return (rust_lib_handle != nullptr) ? JNI_TRUE : JNI_FALSE;
}

/**
 * 获取Rust库加载错误信息
 */
extern "C" JNIEXPORT jstring JNICALL
Java_com_williw_mobile_WilliwNode_getLibraryError(JNIEnv* env, jobject thiz) {
    const char* error = dlerror();
    if (error == nullptr) {
        return env->NewStringUTF("No error");
    }
    return env->NewStringUTF(error);
}
