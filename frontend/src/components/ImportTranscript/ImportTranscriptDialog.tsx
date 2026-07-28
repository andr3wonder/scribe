'use client';

import React, { useState, useRef } from 'react';
import { FileText, Upload, X, Loader2 } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { useConfig } from '@/contexts/ConfigContext';
import { useSidebar } from '@/components/Sidebar/SidebarProvider';
import { storageService } from '@/services/storageService';
import { toast } from 'sonner';
import { useRouter } from 'next/navigation';
import { Transcript } from '@/types';

interface ImportTranscriptDialogProps {
  isOpen: boolean;
  onClose: () => void;
}

export const ImportTranscriptDialog: React.FC<ImportTranscriptDialogProps> = ({
  isOpen,
  onClose,
}) => {
  const [title, setTitle] = useState('');
  const [transcriptText, setTranscriptText] = useState('');
  const [isImporting, setIsImporting] = useState(false);
  const fileInputRef = useRef<HTMLInputElement>(null);
  const { modelConfig } = useConfig();
  const { refetchMeetings } = useSidebar();
  const router = useRouter();

  if (!isOpen) return null;

  const handleFileUpload = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;

    const text = await file.text();
    setTranscriptText(text);

    // Auto-set title from filename if empty
    if (!title) {
      const name = file.name.replace(/\.(txt|md|srt|vtt|json)$/i, '');
      setTitle(name);
    }

    toast.success(`Loaded ${file.name}`);
  };

  const handleImport = async () => {
    if (!transcriptText.trim()) {
      toast.error('Please paste or upload a transcript');
      return;
    }

    setIsImporting(true);

    try {
      // Split text into segments (by paragraphs or double newlines)
      const segments = transcriptText
        .split(/\n\n+/)
        .filter(s => s.trim())
        .map((text, i) => ({
          id: `import-${i}`,
          text: text.trim(),
          timestamp: new Date().toISOString(),
          sequence_id: i,
          chunk_start_time: i * 10, // Fake timestamps
          is_partial: false,
          confidence: 1.0,
          audio_start_time: i * 10,
          audio_end_time: (i + 1) * 10,
          duration: 10,
        } as Transcript));

      // Save as a meeting
      const meetingTitle = title.trim() || 'Imported Transcript';
      const result = await storageService.saveMeeting(meetingTitle, segments, null);
      const meetingId = result.meeting_id;

      console.log('Imported transcript as meeting:', meetingId);

      // Auto-generate summary
      const fullText = segments.map(s => s.text).join('\n');
      toast.info('Generating summary...', { duration: 3000 });

      invoke('api_process_transcript', {
        text: fullText,
        model: modelConfig.provider,
        modelName: modelConfig.model,
        meetingId,
        chunkSize: 40000,
        overlap: 1000,
        customPrompt: '',
        templateId: 'meeting_recap',
      }).catch((err: unknown) => {
        console.warn('Auto-summary for import failed:', err);
      });

      // Refresh meeting list
      await refetchMeetings();

      toast.success('Transcript imported!', {
        description: `${segments.length} segments saved`,
        duration: 5000,
      });

      // Navigate to the meeting
      router.push(`/meeting-details?id=${meetingId}`);

      // Reset and close
      setTitle('');
      setTranscriptText('');
      onClose();
    } catch (error) {
      console.error('Import failed:', error);
      toast.error('Import failed', {
        description: error instanceof Error ? error.message : String(error),
      });
    } finally {
      setIsImporting(false);
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
      <div className="bg-white rounded-xl shadow-2xl w-[600px] max-h-[80vh] flex flex-col">
        {/* Header */}
        <div className="flex items-center justify-between px-6 py-4 border-b">
          <div className="flex items-center gap-2">
            <FileText className="w-5 h-5 text-blue-500" />
            <h2 className="text-lg font-semibold">Import Transcript</h2>
          </div>
          <button
            onClick={onClose}
            className="p-1 hover:bg-gray-100 rounded-full transition-colors"
          >
            <X className="w-5 h-5 text-gray-500" />
          </button>
        </div>

        {/* Body */}
        <div className="flex-1 overflow-y-auto px-6 py-4 space-y-4">
          {/* Title */}
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">
              Meeting Title
            </label>
            <input
              type="text"
              value={title}
              onChange={(e) => setTitle(e.target.value)}
              placeholder="e.g., Team Standup March 29"
              className="w-full px-3 py-2 border rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-400"
            />
          </div>

          {/* File upload */}
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">
              Upload File
            </label>
            <button
              onClick={() => fileInputRef.current?.click()}
              className="flex items-center gap-2 px-4 py-2 border border-dashed border-gray-300 rounded-lg hover:border-blue-400 hover:bg-blue-50 transition-colors text-sm text-gray-600"
            >
              <Upload className="w-4 h-4" />
              Upload .txt, .md, .srt, or .vtt file
            </button>
            <input
              ref={fileInputRef}
              type="file"
              accept=".txt,.md,.srt,.vtt,.json"
              onChange={handleFileUpload}
              className="hidden"
            />
          </div>

          {/* Text area */}
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">
              Or paste transcript text
            </label>
            <textarea
              value={transcriptText}
              onChange={(e) => setTranscriptText(e.target.value)}
              placeholder="Paste your meeting transcript here...&#10;&#10;Each paragraph will become a transcript segment.&#10;Separate segments with blank lines."
              rows={12}
              className="w-full px-3 py-2 border rounded-lg text-sm font-mono focus:outline-none focus:ring-2 focus:ring-blue-400 resize-none"
            />
            {transcriptText && (
              <p className="text-xs text-gray-400 mt-1">
                {transcriptText.split(/\n\n+/).filter(s => s.trim()).length} segments,{' '}
                {transcriptText.split(/\s+/).length} words
              </p>
            )}
          </div>
        </div>

        {/* Footer */}
        <div className="flex items-center justify-end gap-3 px-6 py-4 border-t">
          <button
            onClick={onClose}
            className="px-4 py-2 text-sm text-gray-600 hover:bg-gray-100 rounded-lg transition-colors"
          >
            Cancel
          </button>
          <button
            onClick={handleImport}
            disabled={!transcriptText.trim() || isImporting}
            className="flex items-center gap-2 px-4 py-2 text-sm text-white bg-blue-500 hover:bg-blue-600 disabled:opacity-50 disabled:cursor-not-allowed rounded-lg transition-colors"
          >
            {isImporting ? (
              <>
                <Loader2 className="w-4 h-4 animate-spin" />
                Importing...
              </>
            ) : (
              <>
                <FileText className="w-4 h-4" />
                Import & Summarize
              </>
            )}
          </button>
        </div>
      </div>
    </div>
  );
};
