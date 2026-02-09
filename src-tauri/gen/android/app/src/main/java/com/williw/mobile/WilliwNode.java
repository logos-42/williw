package com.williw.mobile;

import android.content.Context;
import android.util.Log;
import org.json.JSONObject;
import org.json.JSONException;

/**
 * Williw节点包装类 - Java安全版本
 * 提供完整的设备检测功能，避免JNI依赖问题
 */
public class WilliwNode {
    private static final String TAG = "WilliwNode";
    
    // 原生句柄（模拟）
    private long nativeHandle;
    private boolean isInitialized = false;
    
    // 设备能力缓存
    private DeviceCapabilities deviceCapabilities;
    
    /**
     * 构造函数
     */
    public WilliwNode() {
        try {
            // 模拟原生初始化
            nativeHandle = System.currentTimeMillis();
            isInitialized = true;
            Log.d(TAG, "Williw节点初始化成功（Java版本），句柄: " + nativeHandle);
            
            // 初始化默认设备能力
            deviceCapabilities = DeviceCapabilities.getDefault();
        } catch (Exception e) {
            Log.e(TAG, "Williw节点初始化异常: " + e.getMessage(), e);
        }
    }
    
    /**
     * 检查是否已初始化
     */
    public boolean isInitialized() {
        return isInitialized && nativeHandle != 0;
    }
    
    /**
     * 销毁节点
     */
    public void destroy() {
        if (isInitialized && nativeHandle != 0) {
            try {
                nativeHandle = 0;
                isInitialized = false;
                Log.d(TAG, "Williw节点已销毁");
            } catch (Exception e) {
                Log.e(TAG, "销毁Williw节点异常: " + e.getMessage());
            }
        }
    }
    
    /**
     * 获取设备能力
     */
    public DeviceCapabilities getDeviceCapabilities() {
        Log.d(TAG, "开始获取设备能力，初始化状态: " + isInitialized());
        
        if (!isInitialized()) {
            Log.w(TAG, "节点未初始化，返回默认设备能力");
            return DeviceCapabilities.getDefault();
        }
        
        try {
            Log.d(TAG, "返回缓存的设备能力");
            return deviceCapabilities != null ? deviceCapabilities : DeviceCapabilities.getDefault();
        } catch (Exception e) {
            Log.e(TAG, "获取设备能力异常: " + e.getMessage(), e);
            return DeviceCapabilities.getDefault();
        }
    }
    
    /**
     * 更新网络类型
     */
    public boolean updateNetworkType(String networkType) {
        if (!isInitialized()) {
            return false;
        }
        
        try {
            if (deviceCapabilities != null) {
                deviceCapabilities.networkType = networkType;
                Log.d(TAG, "网络类型更新成功: " + networkType);
                return true;
            }
            return false;
        } catch (Exception e) {
            Log.e(TAG, "更新网络类型异常: " + e.getMessage());
            return false;
        }
    }
    
    /**
     * 更新电池信息
     */
    public boolean updateBattery(float batteryLevel, boolean isCharging) {
        if (!isInitialized()) {
            return false;
        }
        
        try {
            if (deviceCapabilities != null) {
                deviceCapabilities.batteryLevel = batteryLevel;
                deviceCapabilities.isCharging = isCharging;
                Log.d(TAG, String.format("电池信息更新成功: 电量=%.2f, 充电=%s", 
                    batteryLevel, isCharging));
                return true;
            }
            return false;
        } catch (Exception e) {
            Log.e(TAG, "更新电池信息异常: " + e.getMessage());
            return false;
        }
    }
    
    /**
     * 更新硬件信息
     */
    public boolean updateHardware(int memoryMb, int cpuCores) {
        if (!isInitialized()) {
            return false;
        }
        
        try {
            if (deviceCapabilities != null) {
                deviceCapabilities.maxMemoryMb = memoryMb;
                deviceCapabilities.cpuCores = cpuCores;
                Log.d(TAG, String.format("硬件信息更新成功: 内存=%dMB, CPU=%d核", 
                    memoryMb, cpuCores));
                return true;
            }
            return false;
        } catch (Exception e) {
            Log.e(TAG, "更新硬件信息异常: " + e.getMessage());
            return false;
        }
    }
    
    /**
     * 获取推荐模型维度
     */
    public int getRecommendedModelDim() {
        if (!isInitialized()) {
            return 256; // 默认值
        }
        
        try {
            if (deviceCapabilities != null) {
                return deviceCapabilities.recommendedModelDim;
            }
            return 256;
        } catch (Exception e) {
            Log.e(TAG, "获取推荐模型维度异常: " + e.getMessage());
            return 256;
        }
    }
    
    /**
     * 获取推荐tick间隔
     */
    public long getRecommendedTickInterval() {
        if (!isInitialized()) {
            return 10; // 默认10秒
        }
        
        try {
            if (deviceCapabilities != null) {
                return deviceCapabilities.recommendedTickInterval;
            }
            return 10;
        } catch (Exception e) {
            Log.e(TAG, "获取推荐tick间隔异常: " + e.getMessage());
            return 10;
        }
    }
    
    /**
     * 是否应该暂停训练
     */
    public boolean shouldPauseTraining() {
        if (!isInitialized()) {
            return false;
        }
        
        try {
            if (deviceCapabilities != null) {
                return deviceCapabilities.shouldPauseTraining();
            }
            return false;
        } catch (Exception e) {
            Log.e(TAG, "检查是否应该暂停训练异常: " + e.getMessage());
            return false;
        }
    }
    
    /**
     * 刷新设备信息
     */
    public boolean refreshDeviceInfo(Context context) {
        if (!isInitialized()) {
            return false;
        }
        
        try {
            // 获取设备信息
            DeviceInfoProvider.DeviceInfo info = DeviceInfoProvider.getDeviceInfo(context);
            
            // 更新设备能力
            if (deviceCapabilities == null) {
                deviceCapabilities = DeviceCapabilities.getDefault();
            }
            
            // 更新硬件信息
            boolean hardwareSuccess = updateHardware((int) info.totalMemoryMb, info.cpuCores);
            
            // 更新网络类型
            boolean networkSuccess = updateNetworkType(info.networkType);
            
            // 更新电池信息
            boolean batterySuccess = updateBattery(info.batteryInfo.level, info.batteryInfo.isCharging);
            
            // 安全更新设备信息 - 使用反射避免编译问题
            try {
                java.lang.reflect.Field brandField = info.getClass().getDeclaredField("brand");
                brandField.setAccessible(true);
                String brand = (String) brandField.get(info);
                deviceCapabilities.deviceBrand = brand != null ? brand : "unknown";
                
                java.lang.reflect.Field modelField = info.getClass().getDeclaredField("model");
                modelField.setAccessible(true);
                String model = (String) modelField.get(info);
                deviceCapabilities.deviceModel = model != null ? model : "unknown";
                
                java.lang.reflect.Field archField = info.getClass().getDeclaredField("architecture");
                archField.setAccessible(true);
                String architecture = (String) archField.get(info);
                deviceCapabilities.cpuArchitecture = architecture != null ? architecture : "unknown";
                
                java.lang.reflect.Field gpuField = info.getClass().getDeclaredField("hasGpu");
                gpuField.setAccessible(true);
                boolean hasGpu = (Boolean) gpuField.get(info);
                deviceCapabilities.hasGpu = hasGpu;
                
            } catch (Exception e) {
                Log.w(TAG, "反射获取设备信息失败，使用默认值: " + e.getMessage());
            }
            
            boolean overallSuccess = hardwareSuccess && networkSuccess && batterySuccess;
            
            if (overallSuccess) {
                Log.d(TAG, "设备信息刷新成功");
            } else {
                Log.w(TAG, "设备信息刷新部分失败: " +
                    "硬件=" + hardwareSuccess + 
                    ", 网络=" + networkSuccess + 
                    ", 电池=" + batterySuccess);
            }
            
            return overallSuccess;
        } catch (Exception e) {
            Log.e(TAG, "刷新设备信息异常: " + e.getMessage(), e);
            return false;
        }
    }
    
    /**
     * 设备能力类
     */
    public static class DeviceCapabilities {
        public String deviceBrand = "unknown";
        public String deviceModel = "unknown";
        public String deviceType = "Unknown";
        public int cpuCores = 4;
        public String cpuArchitecture = "unknown";
        public int maxMemoryMb = 8192;
        public String networkType = "Unknown";
        public boolean hasGpu = false;
        public int recommendedModelDim = 256;
        public long recommendedTickInterval = 10;
        public Float batteryLevel = null;
        public Boolean isCharging = null;
        
        public static DeviceCapabilities getDefault() {
            DeviceCapabilities caps = new DeviceCapabilities();
            caps.deviceBrand = "Williw";
            caps.deviceType = "Phone";
            caps.cpuCores = 8;
            caps.cpuArchitecture = "arm64-v8a";
            caps.maxMemoryMb = 8192;
            caps.networkType = "WiFi";
            caps.hasGpu = true;
            caps.recommendedModelDim = 512;
            caps.recommendedTickInterval = 5;
            caps.batteryLevel = 0.8f;
            caps.isCharging = true;
            return caps;
        }
        
        public static DeviceCapabilities fromJson(String json) {
            try {
                JSONObject obj = new JSONObject(json);
                DeviceCapabilities caps = new DeviceCapabilities();
                caps.deviceBrand = obj.optString("device_brand", "unknown");
                caps.deviceType = obj.optString("device_type", "Unknown");
                caps.cpuCores = obj.optInt("cpu_cores", 4);
                caps.cpuArchitecture = obj.optString("cpu_architecture", "unknown");
                caps.maxMemoryMb = obj.optInt("max_memory_mb", 8192);
                caps.networkType = obj.optString("network_type", "Unknown");
                caps.hasGpu = obj.optBoolean("has_gpu", false);
                caps.recommendedModelDim = obj.optInt("recommended_model_dim", 256);
                caps.recommendedTickInterval = obj.optLong("recommended_tick_interval", 10);
                caps.batteryLevel = obj.has("battery_level") ? (float) obj.getDouble("battery_level") : null;
                caps.isCharging = obj.has("is_charging") ? obj.getBoolean("is_charging") : null;
                return caps;
            } catch (JSONException e) {
                Log.e("DeviceCapabilities", "JSON解析失败: " + e.getMessage());
                return getDefault();
            }
        }
        
        public boolean shouldPauseTraining() {
            if (batteryLevel != null && isCharging != null) {
                return batteryLevel < 0.2 && !isCharging;
            }
            return false;
        }
        
        @Override
        public String toString() {
            return String.format(
                "DeviceCapabilities{brand=%s, type=%s, cpu=%d, arch=%s, memory=%dMB, network=%s, gpu=%s}",
                deviceBrand, deviceType, cpuCores, cpuArchitecture, maxMemoryMb, networkType, hasGpu
            );
        }
    }
}
