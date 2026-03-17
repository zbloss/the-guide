import { useState, useRef, useEffect } from 'react';
import { transcribeAudio } from '../../api/sessions';

// Extend window for browser speech API
interface SpeechRecognitionEvent extends Event {
  resultIndex: number;
  results: SpeechRecognitionResultList;
}

interface SpeechRecognitionErrorEvent extends Event {
  error: string;
}

interface ISpeechRecognition extends EventTarget {
  continuous: boolean;
  interimResults: boolean;
  lang: string;
  onresult: ((event: SpeechRecognitionEvent) => void) | null;
  onerror: ((event: SpeechRecognitionErrorEvent) => void) | null;
  onend: (() => void) | null;
  start(): void;
  stop(): void;
}

interface ISpeechRecognitionConstructor {
  new(): ISpeechRecognition;
}

declare global {
  interface Window {
    SpeechRecognition: ISpeechRecognitionConstructor | undefined;
    webkitSpeechRecognition: ISpeechRecognitionConstructor | undefined;
  }
}

interface VoiceNoteCaptureProps {
  onTranscript: (text: string) => void;
  campaignId: string;
  sessionId: string;
}

export function VoiceNoteCapture({ onTranscript, campaignId, sessionId }: VoiceNoteCaptureProps) {
  const [isRecording, setIsRecording] = useState(false);
  const [transcript, setTranscript] = useState('');
  const [interimTranscript, setInterimTranscript] = useState('');
  const [status, setStatus] = useState('Ready');
  const [useMediaRecorder, setUseMediaRecorder] = useState(false);

  const recognitionRef = useRef<ISpeechRecognition | null>(null);
  const mediaRecorderRef = useRef<MediaRecorder | null>(null);
  const chunksRef = useRef<Blob[]>([]);

  useEffect(() => {
    const SpeechRec = window.SpeechRecognition ?? window.webkitSpeechRecognition;
    if (!SpeechRec) {
      setUseMediaRecorder(true);
    }
  }, []);

  const startSpeechRecognition = () => {
    const SpeechRec = window.SpeechRecognition ?? window.webkitSpeechRecognition;
    if (!SpeechRec) return;

    const recognition = new SpeechRec();
    recognition.continuous = true;
    recognition.interimResults = true;
    recognition.lang = 'en-US';

    recognition.onresult = (event) => {
      let final = '';
      let interim = '';
      for (let i = event.resultIndex; i < event.results.length; i++) {
        const result = event.results[i];
        if (result.isFinal) {
          final += result[0].transcript + ' ';
        } else {
          interim += result[0].transcript;
        }
      }
      if (final) setTranscript((t) => t + final);
      setInterimTranscript(interim);
    };

    recognition.onerror = (event) => {
      setStatus(`Error: ${event.error}`);
      setIsRecording(false);
    };

    recognition.onend = () => {
      setInterimTranscript('');
      if (isRecording) recognition.start(); // restart if still recording
    };

    recognitionRef.current = recognition;
    recognition.start();
    setStatus('Listening…');
    setIsRecording(true);
  };

  const stopSpeechRecognition = () => {
    recognitionRef.current?.stop();
    recognitionRef.current = null;
    setIsRecording(false);
    setStatus('Ready');
    setInterimTranscript('');
  };

  const startMediaRecorder = async () => {
    try {
      const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
      const recorder = new MediaRecorder(stream);
      chunksRef.current = [];
      recorder.ondataavailable = (e) => chunksRef.current.push(e.data);
      recorder.onstop = async () => {
        stream.getTracks().forEach((t) => t.stop());
        const blob = new Blob(chunksRef.current, { type: 'audio/webm' });
        setStatus('Transcribing…');
        try {
          const resp = await transcribeAudio(campaignId, sessionId, blob);
          if (resp.transcript) setTranscript((t) => t + resp.transcript + ' ');
        } catch {
          setStatus('Transcription failed (backend unavailable)');
        }
        setStatus('Ready');
      };
      recorder.start();
      mediaRecorderRef.current = recorder;
      setIsRecording(true);
      setStatus('Recording…');
    } catch (e) {
      setStatus(`Microphone error: ${e}`);
    }
  };

  const stopMediaRecorder = () => {
    mediaRecorderRef.current?.stop();
    mediaRecorderRef.current = null;
    setIsRecording(false);
  };

  const handleToggle = () => {
    if (isRecording) {
      if (useMediaRecorder) stopMediaRecorder();
      else stopSpeechRecognition();
    } else {
      if (useMediaRecorder) void startMediaRecorder();
      else startSpeechRecognition();
    }
  };

  const handleSave = () => {
    const text = transcript.trim();
    if (text) onTranscript(text);
  };

  return (
    <div className="voice-capture" style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
      <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
        <button
          className={`btn ${isRecording ? 'btn-danger' : 'btn-primary'}`}
          onClick={handleToggle}
        >
          {isRecording ? '⏹ Stop Recording' : '🎙 Start Recording'}
        </button>
        <span style={{ color: '#aaa', fontSize: 13 }}>{status}</span>
        {isRecording && (
          <span style={{ color: '#e05a5a', fontSize: 12 }}>● Live</span>
        )}
      </div>

      <textarea
        className="form-input"
        rows={6}
        value={transcript + (interimTranscript ? ` [${interimTranscript}]` : '')}
        onChange={(e) => setTranscript(e.target.value.replace(/ \[.*?\]$/, ''))}
        placeholder="Transcript will appear here as you speak. You can also type or edit."
      />

      <div style={{ display: 'flex', gap: 8 }}>
        <button
          className="btn btn-primary"
          onClick={handleSave}
          disabled={!transcript.trim()}
        >
          Save as Event
        </button>
        <button className="btn" onClick={() => { setTranscript(''); setInterimTranscript(''); }}>
          Clear
        </button>
      </div>

      {useMediaRecorder && (
        <p style={{ fontSize: 12, color: '#aaa', margin: 0 }}>
          Web Speech API unavailable in this browser. Using MediaRecorder with backend transcription.
        </p>
      )}
    </div>
  );
}
