cmake_minimum_required(VERSION 3.18.1)

project("williw_jni")

# 设置C++标准
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# 设置编译选项
set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -Wall -Wextra -O2")
set(CMAKE_CXX_FLAGS_DEBUG "${CMAKE_CXX_FLAGS_DEBUG} -g -DDEBUG")
set(CMAKE_CXX_FLAGS_RELEASE "${CMAKE_CXX_FLAGS_RELEASE} -O3 -DNDEBUG")

# 查找Android日志库
find_library(log-lib log)

# 查找动态链接库
find_library(dl-lib dl)

# 查找OpenGL ES库（用于GPU检测）
find_library(glesv2-lib GLESv2)
find_library(egl-lib EGL)

# 查找Android硬件相关库
find_library(hardware-lib hardware)
find_library(hwui-lib hwui)

# 添加JNI库
add_library(
    williw_jni
    SHARED
    williw_jni.cpp
)

# 链接必要的库
target_link_libraries(
    williw_jni
    ${log-lib}
    ${dl-lib}
    ${glesv2-lib}
    ${egl-lib}
    ${hardware-lib}
    ${hwui-lib}
)

# 包含目录
target_include_directories(
    williw_jni
    PRIVATE
    ${CMAKE_CURRENT_SOURCE_DIR}
)

# 编译定义
target_compile_definitions(
    williw_jni
    PRIVATE
    ANDROID
    __ANDROID_API__=${ANDROID_PLATFORM_LEVEL}
)

# 设置输出目录
set_target_properties(
    williw_jni
    PROPERTIES
    LIBRARY_OUTPUT_DIRECTORY
    ${CMAKE_LIBRARY_OUTPUT_DIRECTORY}
)

# 如果Rust库存在，尝试链接
if(EXISTS ${CMAKE_LIBRARY_OUTPUT_DIRECTORY}/libwilliw.so)
    target_link_libraries(
        williw_jni
        ${CMAKE_LIBRARY_OUTPUT_DIRECTORY}/libwilliw.so
    )
    message(STATUS "Found and linked Rust library: libwilliw.so")
else()
    message(WARNING "Rust library not found: ${CMAKE_LIBRARY_OUTPUT_DIRECTORY}/libwilliw.so")
    message(STATUS "Will attempt to load it at runtime")
endif()

# 打印构建信息
message(STATUS "Building Williw JNI library")
message(STATUS "Android API level: ${ANDROID_PLATFORM_LEVEL}")
message(STATUS "Target ABI: ${ANDROID_ABI}")
message(STATUS "Library output directory: ${CMAKE_LIBRARY_OUTPUT_DIRECTORY}")

# 可选：添加预编译的Rust库支持
option(ENABLE_RUST_STATIC_LINKING "Enable static linking with Rust library" OFF)

if(ENABLE_RUST_STATIC_LINKING)
    # 如果启用静态链接，这里可以添加静态库的配置
    message(STATUS "Static Rust linking enabled")
endif()

# 添加自定义目标用于复制Rust库
if(EXISTS ${CMAKE_SOURCE_DIR}/../../../target/${ANDROID_ABI}/release/libwilliw.so)
    add_custom_target(
        copy_rust_lib ALL
        COMMAND ${CMAKE_COMMAND} -E copy
        ${CMAKE_SOURCE_DIR}/../../../target/${ANDROID_ABI}/release/libwilliw.so
        ${CMAKE_LIBRARY_OUTPUT_DIRECTORY}/libwilliw.so
        COMMENT "Copying Rust library to output directory"
    )
    
    add_dependencies(williw_jni copy_rust_lib)
    message(STATUS "Added Rust library copy target")
endif()

# 安装规则
install(
    TARGETS williw_jni
    LIBRARY DESTINATION lib/${ANDROID_ABI}
)
