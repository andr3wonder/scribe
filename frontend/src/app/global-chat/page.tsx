'use client';

import React from 'react';
import { motion } from 'framer-motion';
import { MessageSquare } from 'lucide-react';
import { ChatPanel } from '@/components/ChatPanel';

export default function GlobalChatPage() {
  return (
    <motion.div
      initial={{ opacity: 0, y: 20 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.3, ease: 'easeOut' }}
      className="flex flex-col h-screen bg-gray-50"
    >
      {/* Page header */}
      <div className="flex-shrink-0 px-6 pt-6 pb-4">
        <div className="flex items-center gap-3">
          <div className="p-2 rounded-lg bg-blue-100">
            <MessageSquare className="w-5 h-5 text-blue-600" />
          </div>
          <div>
            <h1 className="text-xl font-semibold text-gray-800">Global Chat</h1>
            <p className="text-sm text-gray-500">
              Ask questions across all your saved meetings
            </p>
          </div>
        </div>
      </div>

      {/* Chat panel - takes up remaining space */}
      <div className="flex-1 px-6 pb-6 min-h-0">
        <ChatPanel
          isGlobal={true}
          className="h-full rounded-lg shadow-sm border border-gray-200"
        />
      </div>
    </motion.div>
  );
}
