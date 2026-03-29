'use client';

import React, { useState, useEffect, useRef, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Send, Loader2, MessageSquare, Trash2 } from 'lucide-react';
import { useConfig } from '@/contexts/ConfigContext';
import { useTranscripts } from '@/contexts/TranscriptContext';
import { ChatMessage, ChatMessageData } from './ChatMessage';
import { toast } from 'sonner';

interface ChatPanelProps {
  /** Meeting ID for per-meeting chat. If omitted, operates in global mode. */
  meetingId?: string;
  /** Whether this is a global chat (searches all meetings) */
  isGlobal?: boolean;
  /** Optional class name overrides */
  className?: string;
}

export const ChatPanel: React.FC<ChatPanelProps> = ({
  meetingId,
  isGlobal = false,
  className = '',
}) => {
  const [messages, setMessages] = useState<ChatMessageData[]>([]);
  const [inputValue, setInputValue] = useState('');
  const [isLoading, setIsLoading] = useState(false);
  const [isLoadingHistory, setIsLoadingHistory] = useState(false);

  const messagesEndRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLTextAreaElement>(null);
  const scrollContainerRef = useRef<HTMLDivElement>(null);

  const { modelConfig } = useConfig();
  const { transcripts } = useTranscripts();

  // Auto-scroll to bottom when new messages arrive
  const scrollToBottom = useCallback(() => {
    if (messagesEndRef.current) {
      messagesEndRef.current.scrollIntoView({ behavior: 'smooth' });
    }
  }, []);

  useEffect(() => {
    scrollToBottom();
  }, [messages, scrollToBottom]);

  // Load chat history on mount (for saved meetings)
  useEffect(() => {
    if (!meetingId || isGlobal) return;

    const loadHistory = async () => {
      setIsLoadingHistory(true);
      try {
        const history = await invoke<ChatMessageData[]>('get_chat_history', {
          meetingId,
        });
        if (history && history.length > 0) {
          setMessages(history);
        }
      } catch (error) {
        // Chat history might not exist yet for this meeting, that's fine
        console.log('[ChatPanel] No existing chat history for meeting:', meetingId, error);
      } finally {
        setIsLoadingHistory(false);
      }
    };

    loadHistory();
  }, [meetingId, isGlobal]);

  // Load global chat history
  useEffect(() => {
    if (!isGlobal) return;

    const loadGlobalHistory = async () => {
      setIsLoadingHistory(true);
      try {
        const history = await invoke<ChatMessageData[]>('get_global_chat_history');
        if (history && history.length > 0) {
          setMessages(history);
        }
      } catch (error) {
        console.log('[ChatPanel] No existing global chat history:', error);
      } finally {
        setIsLoadingHistory(false);
      }
    };

    loadGlobalHistory();
  }, [isGlobal]);

  // Build transcript text for live recording context
  const getLiveTranscriptText = useCallback((): string => {
    if (!transcripts || transcripts.length === 0) return '';
    return transcripts.map((t) => t.text).join('\n');
  }, [transcripts]);

  const handleSend = async () => {
    const question = inputValue.trim();
    if (!question || isLoading) return;

    // Create user message
    const userMessage: ChatMessageData = {
      id: `user-${Date.now()}`,
      role: 'user',
      content: question,
      timestamp: new Date().toISOString(),
    };

    setMessages((prev) => [...prev, userMessage]);
    setInputValue('');
    setIsLoading(true);

    // Resize textarea back to default
    if (inputRef.current) {
      inputRef.current.style.height = 'auto';
    }

    try {
      let response: string;

      if (isGlobal) {
        // Global chat - search across all meetings
        const result = await invoke<{ answer: string; meeting_id?: string }>('ask_global_question', {
          question,
          modelProvider: modelConfig.provider,
          modelName: modelConfig.model,
        });
        response = result.answer;
      } else if (meetingId) {
        // Per-meeting chat with saved meeting
        const result = await invoke<{ answer: string; meeting_id?: string }>('ask_meeting_question', {
          meetingId,
          question,
          modelProvider: modelConfig.provider,
          modelName: modelConfig.model,
        });
        response = result.answer;
      } else {
        // Live recording mode - pass transcript directly
        const liveTranscript = getLiveTranscriptText();
        if (!liveTranscript) {
          throw new Error('No transcript available yet. Start recording or wait for transcription.');
        }
        const result = await invoke<{ answer: string; meeting_id?: string }>('ask_meeting_question', {
          meetingId: '__live__',
          question,
          modelProvider: modelConfig.provider,
          modelName: modelConfig.model,
          liveTranscript: liveTranscript,
        });
        response = result.answer;
      }

      const assistantMessage: ChatMessageData = {
        id: `assistant-${Date.now()}`,
        role: 'assistant',
        content: response,
        timestamp: new Date().toISOString(),
      };

      setMessages((prev) => [...prev, assistantMessage]);

      // If the response likely contained edits, trigger a refresh of the meeting data
      const lowerQuestion = question.toLowerCase();
      const editKeywords = ['fix', 'correct', 'change', 'update', 'replace', 'rename', 'wrong', 'should be', 'was actually', 'not '];
      if (editKeywords.some(k => lowerQuestion.includes(k))) {
        // Dispatch event so meeting detail page re-fetches transcript + summary
        window.dispatchEvent(new CustomEvent('meeting-data-edited', { detail: { meetingId } }));
      }
    } catch (error) {
      console.error('[ChatPanel] Error asking question:', error);
      const errorMessage: ChatMessageData = {
        id: `error-${Date.now()}`,
        role: 'assistant',
        content: `Sorry, I encountered an error: ${error instanceof Error ? error.message : String(error)}`,
        timestamp: new Date().toISOString(),
      };
      setMessages((prev) => [...prev, errorMessage]);
    } finally {
      setIsLoading(false);
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  };

  // Auto-resize textarea
  const handleInputChange = (e: React.ChangeEvent<HTMLTextAreaElement>) => {
    setInputValue(e.target.value);
    const textarea = e.target;
    textarea.style.height = 'auto';
    textarea.style.height = `${Math.min(textarea.scrollHeight, 120)}px`;
  };

  const handleClearChat = async () => {
    setMessages([]);
    try {
      if (isGlobal) {
        await invoke('clear_global_chat_history');
      } else if (meetingId) {
        await invoke('clear_chat_history', { meetingId });
      }
    } catch (error) {
      // Clearing history is best-effort
      console.warn('[ChatPanel] Failed to clear chat history:', error);
    }
    toast.success('Chat cleared');
  };

  const emptyStateText = isGlobal
    ? 'Ask questions across all your meetings'
    : 'Ask questions about this meeting';

  return (
    <div className={`flex flex-col bg-white border-t border-gray-200 ${className}`}>
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-2 border-b border-gray-100 bg-gray-50">
        <div className="flex items-center gap-2">
          <MessageSquare className="w-4 h-4 text-blue-500" />
          <span className="text-sm font-medium text-gray-700">
            {isGlobal ? 'Global Chat' : 'Meeting Q&A'}
          </span>
          {isLoading && (
            <Loader2 className="w-3.5 h-3.5 text-blue-500 animate-spin" />
          )}
        </div>
        {messages.length > 0 && (
          <button
            onClick={handleClearChat}
            className="p-1 rounded hover:bg-gray-200 transition-colors"
            title="Clear chat"
          >
            <Trash2 className="w-3.5 h-3.5 text-gray-400 hover:text-gray-600" />
          </button>
        )}
      </div>

      {/* Messages area */}
      <div
        ref={scrollContainerRef}
        className="flex-1 overflow-y-auto p-4 min-h-[200px] max-h-[400px]"
      >
        {isLoadingHistory ? (
          <div className="flex items-center justify-center h-full">
            <Loader2 className="w-5 h-5 text-gray-400 animate-spin" />
          </div>
        ) : messages.length === 0 ? (
          <div className="flex flex-col items-center justify-center h-full text-center">
            <MessageSquare className="w-8 h-8 text-gray-300 mb-2" />
            <p className="text-sm text-gray-400">{emptyStateText}</p>
            <p className="text-xs text-gray-300 mt-1">
              {isGlobal
                ? 'Your questions will search across all saved meetings'
                : 'Ask about action items, decisions, or any topic discussed'}
            </p>
          </div>
        ) : (
          <>
            {messages.map((message) => (
              <ChatMessage key={message.id} message={message} />
            ))}
            {isLoading && (
              <div className="flex justify-start mb-3">
                <div className="bg-gray-100 rounded-lg px-4 py-2.5">
                  <div className="flex items-center gap-2">
                    <Loader2 className="w-4 h-4 text-gray-400 animate-spin" />
                    <span className="text-sm text-gray-400">Thinking...</span>
                  </div>
                </div>
              </div>
            )}
            <div ref={messagesEndRef} />
          </>
        )}
      </div>

      {/* Input area */}
      <div className="border-t border-gray-100 p-3">
        <div className="flex items-end gap-2">
          <textarea
            ref={inputRef}
            value={inputValue}
            onChange={handleInputChange}
            onKeyDown={handleKeyDown}
            placeholder={
              isGlobal
                ? 'Ask across all meetings...'
                : 'Ask about this meeting...'
            }
            rows={1}
            disabled={isLoading}
            className="flex-1 resize-none rounded-lg border border-gray-200 bg-gray-50 px-3 py-2 text-sm placeholder:text-gray-400 focus:outline-none focus:ring-1 focus:ring-blue-400 focus:border-blue-400 disabled:opacity-50 disabled:cursor-not-allowed"
            style={{ minHeight: '36px', maxHeight: '120px' }}
          />
          <button
            onClick={handleSend}
            disabled={!inputValue.trim() || isLoading}
            className="flex-shrink-0 p-2 rounded-lg bg-blue-500 text-white hover:bg-blue-600 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
            title="Send message"
          >
            {isLoading ? (
              <Loader2 className="w-4 h-4 animate-spin" />
            ) : (
              <Send className="w-4 h-4" />
            )}
          </button>
        </div>
        <p className="text-[10px] text-gray-300 mt-1 text-center">
          {modelConfig.provider}/{modelConfig.model} -- Press Enter to send, Shift+Enter for new line
        </p>
      </div>
    </div>
  );
};
