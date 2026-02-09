package com.williw.mobile;

import android.view.LayoutInflater;
import android.view.View;
import android.view.ViewGroup;
import android.widget.LinearLayout;
import android.widget.TextView;
import androidx.annotation.NonNull;
import androidx.recyclerview.widget.RecyclerView;
import java.util.List;

public class ChatAdapter extends RecyclerView.Adapter<ChatAdapter.ChatViewHolder> {
    
    private List<ChatMessage> messageList;
    
    public ChatAdapter(List<ChatMessage> messageList) {
        this.messageList = messageList;
    }
    
    @NonNull
    @Override
    public ChatViewHolder onCreateViewHolder(@NonNull ViewGroup parent, int viewType) {
        View view = LayoutInflater.from(parent.getContext())
                .inflate(R.layout.item_chat_message, parent, false);
        return new ChatViewHolder(view);
    }
    
    @Override
    public void onBindViewHolder(@NonNull ChatViewHolder holder, int position) {
        ChatMessage message = messageList.get(position);
        
        if (message.isUser()) {
            // 显示用户消息，隐藏AI消息
            holder.userMessageLayout.setVisibility(View.VISIBLE);
            holder.aiMessageLayout.setVisibility(View.GONE);
            holder.userMessageText.setText(message.getMessage());
        } else {
            // 显示AI消息，隐藏用户消息
            holder.userMessageLayout.setVisibility(View.GONE);
            holder.aiMessageLayout.setVisibility(View.VISIBLE);
            holder.aiMessageText.setText(message.getMessage());
        }
    }
    
    @Override
    public int getItemCount() {
        return messageList.size();
    }
    
    static class ChatViewHolder extends RecyclerView.ViewHolder {
        LinearLayout userMessageLayout;
        LinearLayout aiMessageLayout;
        TextView userMessageText;
        TextView aiMessageText;
        
        public ChatViewHolder(@NonNull View itemView) {
            super(itemView);
            userMessageLayout = itemView.findViewById(R.id.userMessageLayout);
            aiMessageLayout = itemView.findViewById(R.id.aiMessageLayout);
            userMessageText = itemView.findViewById(R.id.userMessageText);
            aiMessageText = itemView.findViewById(R.id.aiMessageText);
        }
    }
}
