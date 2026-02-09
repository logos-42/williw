package com.williw.mobile;

import android.app.ActivityManager;
import android.content.Context;
import android.content.Intent;
import android.content.IntentFilter;
import android.hardware.SensorManager;
import android.net.ConnectivityManager;
import android.net.NetworkInfo;
import android.os.BatteryManager;
import android.os.Build;
import android.os.Environment;
import android.provider.Settings;
import android.telephony.TelephonyManager;
import android.util.Log;
import android.view.Display;
import android.view.WindowManager;

import java.io.BufferedReader;
import java.io.File;
import java.io.FileReader;
import java.io.IOException;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

/**
 * Android设备信息提供者
 * 获取设备的硬件信息、网络状态、电池状态等
 */
public class DeviceInfoProvider {
    private static final String TAG = "DeviceInfoProvider";
    
    /**
     * 设备信息数据类
     */
    public static class DeviceInfo {
        public long totalMemoryMb;
        public int cpuCores;
        public String cpuArchitecture;
        public String networkType;
        public BatteryInfo batteryInfo;
        public String deviceType;
        public String deviceModel;
        public String deviceBrand;
        public String androidVersion;
        public int sdkVersion;
        public boolean hasGpu;
        public String gpuRenderer;
        public boolean hasNpu;
        public String npuInfo;
        
        @Override
        public String toString() {
            return "DeviceInfo{" +
                    "totalMemoryMb=" + totalMemoryMb +
                    ", cpuCores=" + cpuCores +
                    ", cpuArchitecture='" + cpuArchitecture + '\'' +
                    ", networkType='" + networkType + '\'' +
                    ", batteryInfo=" + batteryInfo +
                    ", deviceType='" + deviceType + '\'' +
                    ", deviceModel='" + deviceModel + '\'' +
                    ", deviceBrand='" + deviceBrand + '\'' +
                    ", androidVersion='" + androidVersion + '\'' +
                    ", sdkVersion=" + sdkVersion +
                    ", hasGpu=" + hasGpu +
                    ", gpuRenderer='" + gpuRenderer + '\'' +
                    ", hasNpu=" + hasNpu +
                    ", npuInfo='" + npuInfo + '\'' +
                    '}';
        }
    }
    
    /**
     * 电池信息数据类
     */
    public static class BatteryInfo {
        public float level;          // 0.0-1.0
        public boolean isCharging;
        public int batteryHealth;    // 电池健康状态
        public int batteryTechnology; // 电池技术
        public int temperature;      // 电池温度
        
        @Override
        public String toString() {
            return "BatteryInfo{" +
                    "level=" + level +
                    ", isCharging=" + isCharging +
                    ", batteryHealth=" + batteryHealth +
                    ", batteryTechnology=" + batteryTechnology +
                    ", temperature=" + temperature +
                    '}';
        }
    }
    
    /**
     * 获取完整的设备信息
     * @param context Android上下文
     * @return 设备信息对象
     */
    public static DeviceInfo getDeviceInfo(Context context) {
        DeviceInfo info = new DeviceInfo();
        
        try {
            // 获取基本信息
            info.totalMemoryMb = getTotalMemory(context);
            info.cpuCores = getCpuCores();
            info.cpuArchitecture = getCpuArchitecture();
            info.networkType = getNetworkType(context);
            info.batteryInfo = getBatteryInfo(context);
            info.deviceType = getDeviceType(context);
            info.deviceModel = Build.MODEL;
            info.deviceBrand = Build.BRAND;
            info.androidVersion = Build.VERSION.RELEASE;
            info.sdkVersion = Build.VERSION.SDK_INT;
            
            // 获取GPU信息
            info.hasGpu = hasGpuSupport();
            info.gpuRenderer = getGpuRenderer();
            
            // 获取NPU信息
            info.hasNpu = hasNpuSupport();
            info.npuInfo = getNpuInfo();
            
            Log.d(TAG, "设备信息获取完成: " + info.toString());
            
        } catch (Exception e) {
            Log.e(TAG, "获取设备信息失败: " + e.getMessage());
        }
        
        return info;
    }
    
    /**
     * 获取总内存（MB）
     */
    public static long getTotalMemory(Context context) {
        try {
            ActivityManager activityManager = (ActivityManager) context.getSystemService(Context.ACTIVITY_SERVICE);
            ActivityManager.MemoryInfo memoryInfo = new ActivityManager.MemoryInfo();
            activityManager.getMemoryInfo(memoryInfo);
            
            // 转换为MB
            long totalMemory = memoryInfo.totalMem / (1024 * 1024);
            Log.d(TAG, "总内存: " + totalMemory + " MB");
            return totalMemory;
            
        } catch (Exception e) {
            Log.e(TAG, "获取内存信息失败: " + e.getMessage());
            // 备用方法：从/proc/meminfo读取
            return getMemoryFromProc();
        }
    }
    
    /**
     * 从/proc/meminfo获取内存信息（备用方法）
     */
    private static long getMemoryFromProc() {
        try {
            BufferedReader reader = new BufferedReader(new FileReader("/proc/meminfo"));
            String line = reader.readLine();
            reader.close();
            
            if (line != null) {
                Pattern pattern = Pattern.compile("MemTotal:\\s+(\\d+)\\s+kB");
                Matcher matcher = pattern.matcher(line);
                if (matcher.find()) {
                    long memKb = Long.parseLong(matcher.group(1));
                    long memMb = memKb / 1024;
                    Log.d(TAG, "从/proc/meminfo获取内存: " + memMb + " MB");
                    return memMb;
                }
            }
        } catch (IOException e) {
            Log.e(TAG, "读取/proc/meminfo失败: " + e.getMessage());
        }
        return 2048; // 默认2GB
    }
    
    /**
     * 获取CPU核心数
     */
    public static int getCpuCores() {
        int cores = Runtime.getRuntime().availableProcessors();
        Log.d(TAG, "CPU核心数: " + cores);
        return cores;
    }
    
    /**
     * 获取CPU架构
     */
    public static String getCpuArchitecture() {
        String arch = System.getProperty("os.arch");
        String abi = "";
        
        // 获取主要的ABI
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.LOLLIPOP) {
            abi = Build.SUPPORTED_ABIS[0];
        } else {
            abi = Build.CPU_ABI;
        }
        
        String result = arch + " (" + abi + ")";
        Log.d(TAG, "CPU架构: " + result);
        return result;
    }
    
    /**
     * 获取网络类型
     */
    public static String getNetworkType(Context context) {
        try {
            ConnectivityManager cm = (ConnectivityManager) context.getSystemService(Context.CONNECTIVITY_SERVICE);
            NetworkInfo activeNetwork = cm.getActiveNetworkInfo();
            
            if (activeNetwork != null && activeNetwork.isConnected()) {
                switch (activeNetwork.getType()) {
                    case ConnectivityManager.TYPE_WIFI:
                        Log.d(TAG, "网络类型: WiFi");
                        return "wifi";
                    case ConnectivityManager.TYPE_MOBILE:
                        return getMobileNetworkType(activeNetwork);
                    case ConnectivityManager.TYPE_ETHERNET:
                        Log.d(TAG, "网络类型: 以太网");
                        return "ethernet";
                    default:
                        Log.d(TAG, "网络类型: 未知 (" + activeNetwork.getType() + ")");
                        return "unknown";
                }
            } else {
                Log.d(TAG, "无网络连接");
                return "none";
            }
        } catch (Exception e) {
            Log.e(TAG, "获取网络类型失败: " + e.getMessage());
            return "unknown";
        }
    }
    
    /**
     * 获取移动网络类型详情
     */
    private static String getMobileNetworkType(NetworkInfo networkInfo) {
        // 简化实现，避免TelephonyManager类型转换问题
        String subtypeName = networkInfo.getSubtypeName();
        if (subtypeName != null) {
            subtypeName = subtypeName.toLowerCase();
            if (subtypeName.contains("lte") || subtypeName.contains("4g")) {
                Log.d(TAG, "网络类型: 4G");
                return "4g";
            } else if (subtypeName.contains("nr") || subtypeName.contains("5g")) {
                Log.d(TAG, "网络类型: 5G");
                return "5g";
            } else if (subtypeName.contains("hspa") || subtypeName.contains("hspap") || 
                       subtypeName.contains("hsupa") || subtypeName.contains("hsdpa") || 
                       subtypeName.contains("umts") || subtypeName.contains("3g")) {
                Log.d(TAG, "网络类型: 3G");
                return "3g";
            } else if (subtypeName.contains("edge") || subtypeName.contains("gprs") || 
                       subtypeName.contains("2g")) {
                Log.d(TAG, "网络类型: 2G");
                return "2g";
            }
        }
        Log.d(TAG, "网络类型: 移动网络");
        return "mobile";
    }
    
    /**
     * 获取电池信息
     */
    public static BatteryInfo getBatteryInfo(Context context) {
        try {
            IntentFilter ifilter = new IntentFilter(Intent.ACTION_BATTERY_CHANGED);
            Intent batteryStatus = context.registerReceiver(null, ifilter);
            
            if (batteryStatus != null) {
                BatteryInfo info = new BatteryInfo();
                
                // 电量
                int level = batteryStatus.getIntExtra(BatteryManager.EXTRA_LEVEL, -1);
                int scale = batteryStatus.getIntExtra(BatteryManager.EXTRA_SCALE, -1);
                info.level = (float) level / scale;
                
                // 充电状态
                int status = batteryStatus.getIntExtra(BatteryManager.EXTRA_STATUS, -1);
                info.isCharging = (status == BatteryManager.BATTERY_STATUS_CHARGING || 
                                  status == BatteryManager.BATTERY_STATUS_FULL);
                
                // 电池健康
                info.batteryHealth = batteryStatus.getIntExtra(BatteryManager.EXTRA_HEALTH, -1);
                
                // 电池技术
                info.batteryTechnology = batteryStatus.getIntExtra(BatteryManager.EXTRA_TECHNOLOGY, -1);
                
                // 温度
                info.temperature = batteryStatus.getIntExtra(BatteryManager.EXTRA_TEMPERATURE, -1);
                
                Log.d(TAG, "电池信息: " + info.toString());
                return info;
            }
        } catch (Exception e) {
            Log.e(TAG, "获取电池信息失败: " + e.getMessage());
        }
        return null;
    }
    
    /**
     * 获取设备类型
     */
    public static String getDeviceType(Context context) {
        // 通过屏幕尺寸和内存判断设备类型
        WindowManager wm = (WindowManager) context.getSystemService(Context.WINDOW_SERVICE);
        Display display = wm.getDefaultDisplay();
        
        // 获取屏幕尺寸（英寸）
        double diagonalInches = getScreenSizeInches(display);
        
        // 获取内存
        long memoryMb = getTotalMemory(context);
        
        String deviceType;
        if (diagonalInches >= 7.0 || memoryMb >= 6144) { // 7英寸以上或6GB以上内存
            deviceType = "Tablet";
        } else {
            deviceType = "Phone";
        }
        
        Log.d(TAG, "设备类型: " + deviceType + " (屏幕: " + diagonalInches + "英寸, 内存: " + memoryMb + "MB)");
        return deviceType;
    }
    
    /**
     * 获取屏幕尺寸（英寸）
     */
    private static double getScreenSizeInches(Display display) {
        try {
            android.util.DisplayMetrics metrics = new android.util.DisplayMetrics();
            display.getMetrics(metrics);
            
            int widthPixels = metrics.widthPixels;
            int heightPixels = metrics.heightPixels;
            float density = metrics.density;
            
            float widthInches = widthPixels / density;
            float heightInches = heightPixels / density;
            
            double diagonalInches = Math.sqrt(widthInches * widthInches + heightInches * heightInches);
            return diagonalInches;
        } catch (Exception e) {
            Log.e(TAG, "获取屏幕尺寸失败: " + e.getMessage());
            return 5.5; // 默认5.5英寸
        }
    }
    
    /**
     * 检查GPU支持
     */
    public static boolean hasGpuSupport() {
        try {
            // 检查OpenGL ES支持
            return android.opengl.GLES20.glGetString(android.opengl.GLES20.GL_VERSION) != null;
        } catch (Exception e) {
            Log.e(TAG, "检查GPU支持失败: " + e.getMessage());
            return false;
        }
    }
    
    /**
     * 获取GPU渲染器信息
     */
    public static String getGpuRenderer() {
        try {
            String renderer = android.opengl.GLES20.glGetString(android.opengl.GLES20.GL_RENDERER);
            String vendor = android.opengl.GLES20.glGetString(android.opengl.GLES20.GL_VENDOR);
            String version = android.opengl.GLES20.glGetString(android.opengl.GLES20.GL_VERSION);
            
            String result = vendor + " " + renderer + " (" + version + ")";
            Log.d(TAG, "GPU信息: " + result);
            return result;
        } catch (Exception e) {
            Log.e(TAG, "获取GPU信息失败: " + e.getMessage());
            return "Unknown";
        }
    }
    
    /**
     * 检查NPU支持
     */
    public static boolean hasNpuSupport() {
        try {
            // 检查是否有NNAPI支持
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O_MR1) {
                // Android 8.1+ 支持NNAPI
                return true;
            }
            
            // 检查特定厂商的NPU
            String brand = Build.BRAND.toLowerCase();
            String model = Build.MODEL.toLowerCase();
            
            // 华为麒麟NPU
            if (brand.contains("huawei") || brand.contains("honor")) {
                return model.contains("kirin") && 
                       (model.contains("970") || model.contains("980") || model.contains("990"));
            }
            
            // 高通Hexagon DSP
            if (brand.contains("qualcomm") || brand.contains("xiaomi") || brand.contains("oneplus")) {
                return true; // 大多数高通芯片都有Hexagon DSP
            }
            
            // 三星Exynos NPU
            if (brand.contains("samsung")) {
                return model.contains("exynos") && 
                       (model.contains("9820") || model.contains("9825") || model.contains("990"));
            }
            
            return false;
        } catch (Exception e) {
            Log.e(TAG, "检查NPU支持失败: " + e.getMessage());
            return false;
        }
    }
    
    /**
     * 获取NPU信息
     */
    public static String getNpuInfo() {
        try {
            String brand = Build.BRAND.toLowerCase();
            String model = Build.MODEL.toLowerCase();
            
            if (brand.contains("huawei") || brand.contains("honor")) {
                if (model.contains("kirin 970")) return "Kirin 970 NPU";
                if (model.contains("kirin 980")) return "Kirin 980 NPU";
                if (model.contains("kirin 990")) return "Kirin 990 NPU";
            }
            
            if (brand.contains("qualcomm") || brand.contains("xiaiaomi") || brand.contains("oneplus")) {
                return "Qualcomm Hexagon DSP";
            }
            
            if (brand.contains("samsung")) {
                if (model.contains("exynos 9820")) return "Exynos 9820 NPU";
                if (model.contains("exynos 9825")) return "Exynos 9825 NPU";
                if (model.contains("exynos 990")) return "Exynos 990 NPU";
            }
            
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O_MR1) {
                return "Android NNAPI";
            }
            
            return "None";
        } catch (Exception e) {
            Log.e(TAG, "获取NPU信息失败: " + e.getMessage());
            return "Unknown";
        }
    }
}
