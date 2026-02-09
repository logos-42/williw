package com.williw.mobile;

import android.Manifest;
import android.content.pm.PackageManager;
import android.os.Build;
import android.os.Bundle;
import android.os.Handler;
import android.os.Looper;
import android.util.Log;
import android.view.View;
import android.widget.EditText;
import android.widget.ImageButton;
import android.widget.TextView;
import android.widget.Toast;
import androidx.appcompat.app.AppCompatActivity;
import androidx.core.app.ActivityCompat;
import androidx.core.content.ContextCompat;
import androidx.recyclerview.widget.LinearLayoutManager;
import androidx.recyclerview.widget.RecyclerView;
import java.util.ArrayList;
import java.util.List;

public class MainActivity extends AppCompatActivity {
    private static final String TAG = "MainActivity";
    private static final int PERMISSION_REQUEST_CODE = 1001;
    
    // UI组件
    private EditText messageEditText;
    private ImageButton sendButton;
    private RecyclerView chatRecyclerView;
    private ChatAdapter chatAdapter;
    private List<ChatMessage> messageList;
    private TextView deviceInfoTextView;
    
    // Williw节点实例
    private WilliwNode williwNode;
    
    // 设备信息更新
    private Handler deviceUpdateHandler;
    private Runnable deviceUpdateRunnable;
    private static final long DEVICE_UPDATE_INTERVAL = 30000; // 30秒更新一次
    
    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        setContentView(R.layout.activity_main);
        
        Log.d(TAG, "MainActivity onCreate开始");
        
        // 检查权限
        checkAndRequestPermissions();
        
        // 初始化UI组件
        initViews();
        
        // 设置RecyclerView
        setupRecyclerView();
        
        // 初始化Williw节点
        initWilliwNode();
        
        // 设置点击监听器
        setupListeners();
        
        // 初始化设备信息更新
        initDeviceInfoUpdater();
        
        // 添加欢迎消息
        addWelcomeMessage();
        
        // 显示初始设备信息
        updateDeviceInfoDisplay();
        
        Log.d(TAG, "MainActivity onCreate完成");
    }
    
    private void initViews() {
        messageEditText = findViewById(R.id.messageEditText);
        sendButton = findViewById(R.id.sendButton);
        chatRecyclerView = findViewById(R.id.chatRecyclerView);
        deviceInfoTextView = findViewById(R.id.deviceInfoTextView);
    }
    
    /**
     * 检查和请求权限
     */
    private void checkAndRequestPermissions() {
        String[] permissions = {
            Manifest.permission.ACCESS_NETWORK_STATE,
            Manifest.permission.ACCESS_WIFI_STATE,
            Manifest.permission.READ_PHONE_STATE
        };
        
        List<String> permissionsToRequest = new ArrayList<>();
        for (String permission : permissions) {
            if (ContextCompat.checkSelfPermission(this, permission) != PackageManager.PERMISSION_GRANTED) {
                permissionsToRequest.add(permission);
            }
        }
        
        if (!permissionsToRequest.isEmpty()) {
            ActivityCompat.requestPermissions(this, permissionsToRequest.toArray(new String[0]), PERMISSION_REQUEST_CODE);
        }
    }
    
    /**
     * 初始化Williw节点
     */
    private void initWilliwNode() {
        try {
            Log.d(TAG, "开始初始化Williw节点");
            williwNode = new WilliwNode();
            
            if (williwNode != null && williwNode.isInitialized()) {
                Log.d(TAG, "Williw节点初始化成功");
                
                // 刷新设备信息
                refreshDeviceInfo();
                
                // 显示设备信息
                updateDeviceInfoDisplay();
            } else {
                Log.e(TAG, "Williw节点初始化失败");
                Toast.makeText(this, "Williw节点初始化失败", Toast.LENGTH_LONG).show();
            }
        } catch (Exception e) {
            Log.e(TAG, "初始化Williw节点时发生异常: " + e.getMessage());
            Toast.makeText(this, "初始化失败: " + e.getMessage(), Toast.LENGTH_LONG).show();
        }
    }
    
    /**
     * 初始化设备信息更新器
     */
    private void initDeviceInfoUpdater() {
        deviceUpdateHandler = new Handler(Looper.getMainLooper());
        deviceUpdateRunnable = new Runnable() {
            @Override
            public void run() {
                if (williwNode != null && williwNode.isInitialized()) {
                    refreshDeviceInfo();
                    updateDeviceInfoDisplay();
                }
                // 重复执行
                deviceUpdateHandler.postDelayed(this, DEVICE_UPDATE_INTERVAL);
            }
        };
        
        // 开始定期更新
        deviceUpdateHandler.postDelayed(deviceUpdateRunnable, DEVICE_UPDATE_INTERVAL);
    }
    
    /**
     * 刷新设备信息
     */
    private void refreshDeviceInfo() {
        if (williwNode != null && williwNode.isInitialized()) {
            boolean success = williwNode.refreshDeviceInfo(this);
            if (success) {
                Log.d(TAG, "设备信息刷新成功");
            } else {
                Log.w(TAG, "设备信息刷新失败");
            }
        }
    }
    
    /**
     * 更新设备信息显示
     */
    private void updateDeviceInfoDisplay() {
        Log.d(TAG, "开始更新设备信息显示");
        
        try {
            if (williwNode != null) {
                Log.d(TAG, "WilliwNode不为空，检查初始化状态: " + (williwNode != null ? "非空" : "空"));
                
                if (williwNode.isInitialized()) {
                    Log.d(TAG, "WilliwNode已初始化，获取设备能力");
                    WilliwNode.DeviceCapabilities caps = williwNode.getDeviceCapabilities();
                    
                    if (caps != null && deviceInfoTextView != null) {
                        Log.d(TAG, "设备能力获取成功，更新显示");
                        String info = String.format(
                            "设备: %s %s\n" +
                            "CPU: %d核 %s\n" +
                            "内存: %dMB\n" +
                            "网络: %s\n" +
                            "GPU: %s\n" +
                            "推荐模型: %d维\n" +
                            "训练间隔: %ds",
                            caps.deviceBrand,
                            caps.deviceType,
                            caps.cpuCores,
                            caps.cpuArchitecture,
                            caps.maxMemoryMb,
                            caps.networkType,
                            caps.hasGpu ? "支持" : "不支持",
                            caps.recommendedModelDim,
                            caps.recommendedTickInterval
                        );
                        
                        Log.d(TAG, "设备信息: " + info);
                        runOnUiThread(() -> {
                            deviceInfoTextView.setText(info);
                            Log.d(TAG, "设备信息已更新到UI");
                        });
                    } else {
                        Log.e(TAG, "设备能力为空或TextView为空");
                    }
                } else {
                    Log.w(TAG, "WilliwNode未初始化，显示默认信息");
                    if (deviceInfoTextView != null) {
                        runOnUiThread(() -> {
                            deviceInfoTextView.setText("设备检测中...");
                            Log.d(TAG, "显示默认信息");
                        });
                    }
                }
            } else {
                Log.e(TAG, "WilliwNode为空");
                if (deviceInfoTextView != null) {
                    runOnUiThread(() -> {
                        deviceInfoTextView.setText("Williw节点未初始化");
                        Log.d(TAG, "显示节点未初始化");
                    });
                }
            }
        } catch (Exception e) {
            Log.e(TAG, "更新设备信息显示时发生异常: " + e.getMessage());
        }
    }
    
    private void setupRecyclerView() {
        messageList = new ArrayList<>();
        chatAdapter = new ChatAdapter(messageList);
        chatRecyclerView.setLayoutManager(new LinearLayoutManager(this));
        chatRecyclerView.setAdapter(chatAdapter);
    }
    
    private void setupListeners() {
        sendButton.setOnClickListener(v -> sendMessage());
        
        // 监听EditText文本变化，动态启用/禁用发送按钮
        messageEditText.addTextChangedListener(new android.text.TextWatcher() {
            @Override
            public void beforeTextChanged(CharSequence s, int start, int count, int after) {}
            
            @Override
            public void onTextChanged(CharSequence s, int start, int before, int count) {
                sendButton.setEnabled(s.toString().trim().length() > 0);
            }
            
            @Override
            public void afterTextChanged(android.text.Editable s) {}
        });
    }
    
    private void sendMessage() {
        String message = messageEditText.getText().toString().trim();
        if (!message.isEmpty()) {
            // 添加用户消息
            messageList.add(new ChatMessage(message, ChatMessage.TYPE_USER));
            chatAdapter.notifyItemInserted(messageList.size() - 1);
            
            // 清空输入框
            messageEditText.setText("");
            
            // 生成AI回复
            String aiResponse = generateAIResponse(message);
            
            // 添加AI回复
            messageList.add(new ChatMessage(aiResponse, ChatMessage.TYPE_AI));
            chatAdapter.notifyItemInserted(messageList.size() - 1);
            
            // 滚动到底部
            chatRecyclerView.smoothScrollToPosition(messageList.size() - 1);
        }
    }
    
    private void addWelcomeMessage() {
        String welcomeMessage = "欢迎使用Williw AI助手！我可以帮助您了解设备信息和去中心化训练的相关内容。";
        messageList.add(new ChatMessage(welcomeMessage, ChatMessage.TYPE_AI));
        chatAdapter.notifyItemInserted(0);
    }
    
    private String generateAIResponse(String userMessage) {
        // 增强的AI回复逻辑，包含设备信息相关回答
        if (userMessage.contains("你好") || userMessage.contains("hi") || userMessage.contains("hello")) {
            return "你好！我是Williw AI助手，很高兴为您服务！我可以帮助您了解去中心化训练的相关内容，以及您的设备能力信息。";
        } else if (userMessage.contains("设备") || userMessage.contains("device")) {
            if (williwNode != null && williwNode.isInitialized()) {
                WilliwNode.DeviceCapabilities caps = williwNode.getDeviceCapabilities();
                if (caps != null) {
                    return String.format(
                        "您的设备信息：\n" +
                        "设备类型：%s %s\n" +
                        "CPU：%d核 %s\n" +
                        "内存：%dMB\n" +
                        "网络：%s\n" +
                        "GPU支持：%s\n" +
                        "推荐模型维度：%d\n" +
                        "推荐训练间隔：%d秒",
                        caps.deviceBrand,
                        caps.deviceType,
                        caps.cpuCores,
                        caps.cpuArchitecture,
                        caps.maxMemoryMb,
                        caps.networkType,
                        caps.hasGpu ? "支持" : "不支持",
                        caps.recommendedModelDim,
                        caps.recommendedTickInterval
                    );
                }
            }
            return "设备信息正在获取中，请稍后再试...";
        } else if (userMessage.contains("状态") || userMessage.contains("status")) {
            if (williwNode != null && williwNode.isInitialized()) {
                WilliwNode.DeviceCapabilities caps = williwNode.getDeviceCapabilities();
                if (caps != null) {
                    return String.format(
                        "系统状态：\n" +
                        "设备：%s %s\n" +
                        "性能评分：%.2f\n" +
                        "电池状态：%s\n" +
                        "训练状态：%s",
                        caps.deviceBrand + " " + caps.deviceType,
                        0.85, // 临时性能评分
                        caps.batteryLevel != null ? String.format("%.0f%%", caps.batteryLevel * 100) : "未知",
                        caps.shouldPauseTraining() ? "暂停" : "运行中"
                    );
                }
            }
            return "状态信息获取中...";
        } else {
            return "我是Williw AI助手，您可以询问我关于设备信息、训练状态或其他相关问题。";
        }
    }
}
