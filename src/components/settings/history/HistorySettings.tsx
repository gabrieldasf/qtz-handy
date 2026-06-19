import React, { useCallback, useEffect, useRef, useState } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import { readFile } from "@tauri-apps/plugin-fs";
import {
  ArrowRight,
  Check,
  Copy,
  FolderOpen,
  Plus,
  RotateCcw,
  Star,
  Trash2,
  X,
} from "lucide-react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import {
  commands,
  events,
  type HistoryEntry,
  type HistoryUpdatePayload,
} from "@/bindings";
import { useOsType } from "@/hooks/useOsType";
import { formatDateTime } from "@/utils/dateFormat";
import { AudioPlayer } from "../../ui/AudioPlayer";
import { Button } from "../../ui/Button";
import { Input } from "../../ui/Input";

const IconButton: React.FC<{
  onClick: () => void;
  title: string;
  disabled?: boolean;
  active?: boolean;
  children: React.ReactNode;
}> = ({ onClick, title, disabled, active, children }) => (
  <button
    onClick={onClick}
    disabled={disabled}
    className={`p-1.5 rounded-md flex items-center justify-center transition-colors cursor-pointer disabled:cursor-not-allowed disabled:text-text/20 ${
      active
        ? "text-logo-primary hover:text-logo-primary/80"
        : "text-text/50 hover:text-logo-primary"
    }`}
    title={title}
  >
    {children}
  </button>
);

const PAGE_SIZE = 30;
const CORRECTION_ADVANCE_KEYS = new Set(["Tab", "Enter", "ArrowRight"]);

const shouldAdvanceCorrectionField = (
  event: React.KeyboardEvent<HTMLInputElement>,
) => CORRECTION_ADVANCE_KEYS.has(event.key) && !event.shiftKey;

interface CorrectionDraft {
  id: string;
  heardText: string;
  correctText: string;
  status: "editing" | "saving" | "saved";
}

const createEmptyCorrectionDraft = (): CorrectionDraft => ({
  id:
    typeof crypto !== "undefined" && "randomUUID" in crypto
      ? crypto.randomUUID()
      : `${Date.now()}-${Math.random()}`,
  heardText: "",
  correctText: "",
  status: "editing",
});

const normalizeCorrectionText = (value: string) =>
  value.trim().replace(/\s+/g, " ").toLowerCase();

const createCorrectionRule = async (
  heardText: string,
  correctText: string,
  historyEntryId: number,
) => {
  const result = await commands.createCorrectionRule(
    heardText,
    correctText,
    historyEntryId,
  );

  if (result.status === "error") {
    throw new Error(result.error);
  }
};

interface OpenRecordingsButtonProps {
  onClick: () => void;
  label: string;
}

const OpenRecordingsButton: React.FC<OpenRecordingsButtonProps> = ({
  onClick,
  label,
}) => (
  <Button
    onClick={onClick}
    variant="secondary"
    size="sm"
    className="flex items-center gap-2"
    title={label}
  >
    <FolderOpen className="w-4 h-4" />
    <span>{label}</span>
  </Button>
);

interface CorrectionRowEditorProps {
  entryId: number;
  disabled: boolean;
}

const CorrectionRowEditor: React.FC<CorrectionRowEditorProps> = ({
  entryId,
  disabled,
}) => {
  const { t } = useTranslation();
  const [drafts, setDrafts] = useState<CorrectionDraft[]>([]);
  const heardInputRefs = useRef<Record<string, HTMLInputElement | null>>({});
  const correctInputRefs = useRef<Record<string, HTMLInputElement | null>>({});
  const pendingFocusRef = useRef<{
    draftId: string;
    field: "heard" | "correct";
  } | null>(null);

  useEffect(() => {
    const pendingFocus = pendingFocusRef.current;
    if (!pendingFocus) return;

    const refMap =
      pendingFocus.field === "heard" ? heardInputRefs : correctInputRefs;
    const input = refMap.current[pendingFocus.draftId];
    if (input) {
      input.focus();
      pendingFocusRef.current = null;
    }
  }, [drafts]);

  const setDraftValue = (
    draftId: string,
    field: "heardText" | "correctText",
    value: string,
  ) => {
    setDrafts((currentDrafts) =>
      currentDrafts.map((draft) =>
        draft.id === draftId ? { ...draft, [field]: value } : draft,
      ),
    );
  };

  const hasDuplicateDraft = (draftToCheck: CorrectionDraft) => {
    const normalizedHeard = normalizeCorrectionText(draftToCheck.heardText);

    return drafts.some(
      (draft) =>
        draft.id !== draftToCheck.id &&
        draft.status === "saved" &&
        normalizeCorrectionText(draft.heardText) === normalizedHeard,
    );
  };

  const addCorrectionRow = () => {
    const existingActiveDraft = drafts.find(
      (draft) => draft.status !== "saved",
    );

    if (existingActiveDraft) {
      pendingFocusRef.current = {
        draftId: existingActiveDraft.id,
        field: "heard",
      };
      heardInputRefs.current[existingActiveDraft.id]?.focus();
      return;
    }

    const draft = createEmptyCorrectionDraft();
    pendingFocusRef.current = { draftId: draft.id, field: "heard" };
    setDrafts((currentDrafts) => [...currentDrafts, draft]);
  };

  const cancelDraft = (draftId: string) => {
    setDrafts((currentDrafts) =>
      currentDrafts.filter((draft) => draft.id !== draftId),
    );
  };

  const commitDraft = async (draft: CorrectionDraft) => {
    const heardText = draft.heardText.trim();
    const correctText = draft.correctText.trim();

    if (!heardText || !correctText) {
      toast.error(t("settings.history.corrections.partialRow"));
      return;
    }

    if (hasDuplicateDraft(draft)) {
      toast.error(
        t("settings.history.corrections.duplicate", {
          heardText,
        }),
      );
      return;
    }

    if (draft.status !== "editing") {
      return;
    }

    setDrafts((currentDrafts) =>
      currentDrafts.map((currentDraft) =>
        currentDraft.id === draft.id
          ? { ...currentDraft, heardText, correctText, status: "saving" }
          : currentDraft,
      ),
    );

    try {
      await createCorrectionRule(heardText, correctText, entryId);
      const nextDraft = createEmptyCorrectionDraft();
      pendingFocusRef.current = { draftId: nextDraft.id, field: "heard" };
      setDrafts((currentDrafts) => [
        ...currentDrafts.map((currentDraft) =>
          currentDraft.id === draft.id
            ? {
                ...currentDraft,
                heardText,
                correctText,
                status: "saved" as const,
              }
            : currentDraft,
        ),
        nextDraft,
      ]);
      toast.success(t("settings.history.corrections.created"));
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      console.error("Failed to create correction rule:", error);
      setDrafts((currentDrafts) =>
        currentDrafts.map((currentDraft) =>
          currentDraft.id === draft.id
            ? { ...currentDraft, status: "editing" }
            : currentDraft,
        ),
      );
      toast.error(
        message.toLocaleLowerCase().includes("duplicate") ||
          message.toLocaleLowerCase().includes("already")
          ? t("settings.history.corrections.duplicate", {
              heardText,
            })
          : t("settings.history.corrections.createError"),
      );
    }
  };

  const handleHeardKeyDown = (
    event: React.KeyboardEvent<HTMLInputElement>,
    draft: CorrectionDraft,
  ) => {
    if (event.key === "Escape") {
      event.preventDefault();
      cancelDraft(draft.id);
      return;
    }

    if (!shouldAdvanceCorrectionField(event)) return;
    if (!draft.heardText.trim()) return;

    event.preventDefault();
    correctInputRefs.current[draft.id]?.focus();
  };

  const handleCorrectKeyDown = (
    event: React.KeyboardEvent<HTMLInputElement>,
    draft: CorrectionDraft,
  ) => {
    if (event.key === "Escape") {
      event.preventDefault();
      cancelDraft(draft.id);
      return;
    }

    if (!shouldAdvanceCorrectionField(event)) return;

    event.preventDefault();
    void commitDraft(draft);
  };

  return (
    <div className="space-y-2">
      <Button
        type="button"
        onClick={addCorrectionRow}
        disabled={disabled}
        variant="secondary"
        size="sm"
        className="inline-flex items-center gap-1.5"
        title={t("settings.history.corrections.add")}
      >
        <Plus className="w-3.5 h-3.5" />
        <span>{t("settings.history.corrections.add")}</span>
      </Button>

      {drafts.length > 0 && (
        <div className="space-y-1.5">
          {drafts.map((draft) => {
            const isSaved = draft.status === "saved";
            const isSaving = draft.status === "saving";
            return (
              <div
                key={draft.id}
                className="grid grid-cols-[minmax(0,1fr)_auto] items-center gap-2 sm:grid-cols-[minmax(0,1fr)_auto_minmax(0,1fr)_auto]"
              >
                <Input
                  ref={(element) => {
                    heardInputRefs.current[draft.id] = element;
                  }}
                  type="text"
                  value={draft.heardText}
                  onChange={(event) =>
                    setDraftValue(draft.id, "heardText", event.target.value)
                  }
                  onKeyDown={(event) => handleHeardKeyDown(event, draft)}
                  placeholder={t(
                    "settings.history.corrections.heardPlaceholder",
                  )}
                  aria-label={t("settings.history.corrections.heardLabel")}
                  variant="compact"
                  disabled={disabled || isSaved || isSaving}
                  className="w-full min-w-0"
                />
                <ArrowRight
                  className="w-3.5 h-3.5 text-text/40"
                  aria-hidden="true"
                />
                <Input
                  ref={(element) => {
                    correctInputRefs.current[draft.id] = element;
                  }}
                  type="text"
                  value={draft.correctText}
                  onChange={(event) =>
                    setDraftValue(draft.id, "correctText", event.target.value)
                  }
                  onKeyDown={(event) => handleCorrectKeyDown(event, draft)}
                  placeholder={t(
                    "settings.history.corrections.correctPlaceholder",
                  )}
                  aria-label={t("settings.history.corrections.correctLabel")}
                  variant="compact"
                  disabled={disabled || isSaved || isSaving}
                  className="w-full min-w-0 col-start-1 sm:col-start-auto"
                />
                {isSaved ? (
                  <Check
                    className="w-4 h-4 text-logo-primary"
                    aria-label={t("settings.history.corrections.saved")}
                  />
                ) : (
                  <IconButton
                    onClick={() => cancelDraft(draft.id)}
                    disabled={disabled || isSaving}
                    title={t("settings.history.corrections.cancel")}
                  >
                    <X width={16} height={16} />
                  </IconButton>
                )}
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
};

export const HistorySettings: React.FC = () => {
  const { t } = useTranslation();
  const osType = useOsType();
  const [entries, setEntries] = useState<HistoryEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [hasMore, setHasMore] = useState(true);
  const sentinelRef = useRef<HTMLDivElement>(null);
  const entriesRef = useRef<HistoryEntry[]>([]);
  const loadingRef = useRef(false);

  // Keep ref in sync for use in IntersectionObserver callback
  useEffect(() => {
    entriesRef.current = entries;
  }, [entries]);

  const loadPage = useCallback(async (cursor?: number) => {
    const isFirstPage = cursor === undefined;
    if (!isFirstPage && loadingRef.current) return;
    loadingRef.current = true;

    if (isFirstPage) setLoading(true);

    try {
      const result = await commands.getHistoryEntries(
        cursor ?? null,
        PAGE_SIZE,
      );
      if (result.status === "ok") {
        const { entries: newEntries, has_more } = result.data;
        setEntries((prev) =>
          isFirstPage ? newEntries : [...prev, ...newEntries],
        );
        setHasMore(has_more);
      }
    } catch (error) {
      console.error("Failed to load history entries:", error);
    } finally {
      setLoading(false);
      loadingRef.current = false;
    }
  }, []);

  // Initial load
  useEffect(() => {
    loadPage();
  }, [loadPage]);

  // Infinite scroll via IntersectionObserver
  useEffect(() => {
    if (loading) return;

    const sentinel = sentinelRef.current;
    if (!sentinel || !hasMore) return;

    const observer = new IntersectionObserver(
      (observerEntries) => {
        const first = observerEntries[0];
        if (first.isIntersecting) {
          const lastEntry = entriesRef.current[entriesRef.current.length - 1];
          if (lastEntry) {
            loadPage(lastEntry.id);
          }
        }
      },
      { threshold: 0 },
    );

    observer.observe(sentinel);
    return () => observer.disconnect();
  }, [loading, hasMore, loadPage]);

  // Listen for new entries added from the transcription pipeline
  useEffect(() => {
    const unlisten = events.historyUpdatePayload.listen((event) => {
      const payload: HistoryUpdatePayload = event.payload;
      if (payload.action === "added") {
        setEntries((prev) => [payload.entry, ...prev]);
      } else if (payload.action === "updated") {
        setEntries((prev) =>
          prev.map((e) => (e.id === payload.entry.id ? payload.entry : e)),
        );
      }
      // "deleted" and "toggled" are handled by optimistic updates only,
      // so we intentionally ignore them here to avoid double-mutation.
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  const toggleSaved = async (id: number) => {
    // Optimistic update
    setEntries((prev) =>
      prev.map((e) => (e.id === id ? { ...e, saved: !e.saved } : e)),
    );
    try {
      const result = await commands.toggleHistoryEntrySaved(id);
      if (result.status !== "ok") {
        // Revert on failure
        setEntries((prev) =>
          prev.map((e) => (e.id === id ? { ...e, saved: !e.saved } : e)),
        );
      }
    } catch (error) {
      console.error("Failed to toggle saved status:", error);
      // Revert on failure
      setEntries((prev) =>
        prev.map((e) => (e.id === id ? { ...e, saved: !e.saved } : e)),
      );
    }
  };

  const copyToClipboard = async (text: string) => {
    try {
      await navigator.clipboard.writeText(text);
    } catch (error) {
      console.error("Failed to copy to clipboard:", error);
    }
  };

  const getAudioUrl = useCallback(
    async (fileName: string) => {
      try {
        const result = await commands.getAudioFilePath(fileName);
        if (result.status === "ok") {
          if (osType === "linux") {
            const fileData = await readFile(result.data);
            const blob = new Blob([fileData], { type: "audio/wav" });
            return URL.createObjectURL(blob);
          }
          return convertFileSrc(result.data, "asset");
        }
        return null;
      } catch (error) {
        console.error("Failed to get audio file path:", error);
        return null;
      }
    },
    [osType],
  );

  const deleteAudioEntry = async (id: number) => {
    // Optimistically remove
    setEntries((prev) => prev.filter((e) => e.id !== id));
    try {
      const result = await commands.deleteHistoryEntry(id);
      if (result.status !== "ok") {
        // Reload on failure
        loadPage();
      }
    } catch (error) {
      console.error("Failed to delete entry:", error);
      loadPage();
    }
  };

  const retryHistoryEntry = async (id: number) => {
    const result = await commands.retryHistoryEntryTranscription(id);
    if (result.status !== "ok") {
      throw new Error(String(result.error));
    }
  };

  const openRecordingsFolder = async () => {
    try {
      const result = await commands.openRecordingsFolder();
      if (result.status !== "ok") {
        throw new Error(String(result.error));
      }
    } catch (error) {
      console.error("Failed to open recordings folder:", error);
    }
  };

  let content: React.ReactNode;

  if (loading) {
    content = (
      <div className="px-4 py-3 text-center text-text/60">
        {t("settings.history.loading")}
      </div>
    );
  } else if (entries.length === 0) {
    content = (
      <div className="px-4 py-3 text-center text-text/60">
        {t("settings.history.empty")}
      </div>
    );
  } else {
    content = (
      <>
        <div className="divide-y divide-mid-gray/20">
          {entries.map((entry) => (
            <HistoryEntryComponent
              key={entry.id}
              entry={entry}
              onToggleSaved={() => toggleSaved(entry.id)}
              onCopyText={() => copyToClipboard(entry.transcription_text)}
              getAudioUrl={getAudioUrl}
              deleteAudio={deleteAudioEntry}
              retryTranscription={retryHistoryEntry}
            />
          ))}
        </div>
        {/* Sentinel for infinite scroll */}
        <div ref={sentinelRef} className="h-1" />
      </>
    );
  }

  return (
    <div className="max-w-3xl w-full mx-auto space-y-6">
      <div className="space-y-2">
        <div className="px-4 flex items-center justify-between">
          <div>
            <h2 className="text-xs font-medium text-mid-gray uppercase tracking-wide">
              {t("settings.history.title")}
            </h2>
          </div>
          <OpenRecordingsButton
            onClick={openRecordingsFolder}
            label={t("settings.history.openFolder")}
          />
        </div>
        <div className="bg-background border border-mid-gray/20 rounded-lg overflow-visible">
          {content}
        </div>
      </div>
    </div>
  );
};

interface HistoryEntryProps {
  entry: HistoryEntry;
  onToggleSaved: () => void;
  onCopyText: () => void;
  getAudioUrl: (fileName: string) => Promise<string | null>;
  deleteAudio: (id: number) => Promise<void>;
  retryTranscription: (id: number) => Promise<void>;
}

const HistoryEntryComponent: React.FC<HistoryEntryProps> = ({
  entry,
  onToggleSaved,
  onCopyText,
  getAudioUrl,
  deleteAudio,
  retryTranscription,
}) => {
  const { t, i18n } = useTranslation();
  const [showCopied, setShowCopied] = useState(false);
  const [retrying, setRetrying] = useState(false);

  const hasTranscription = entry.transcription_text.trim().length > 0;

  const handleLoadAudio = useCallback(
    () => getAudioUrl(entry.file_name),
    [getAudioUrl, entry.file_name],
  );

  const handleCopyText = () => {
    if (!hasTranscription) {
      return;
    }

    onCopyText();
    setShowCopied(true);
    setTimeout(() => setShowCopied(false), 2000);
  };

  const handleDeleteEntry = async () => {
    try {
      await deleteAudio(entry.id);
    } catch (error) {
      console.error("Failed to delete entry:", error);
      toast.error(t("settings.history.deleteError"));
    }
  };

  const handleRetranscribe = async () => {
    try {
      setRetrying(true);
      await retryTranscription(entry.id);
    } catch (error) {
      console.error("Failed to re-transcribe:", error);
      toast.error(t("settings.history.retranscribeError"));
    } finally {
      setRetrying(false);
    }
  };

  const formattedDate = formatDateTime(String(entry.timestamp), i18n.language);

  return (
    <div className="px-4 py-2 pb-5 flex flex-col gap-3">
      <div className="flex justify-between items-center">
        <p className="text-sm font-medium">{formattedDate}</p>
        <div className="flex items-center">
          <IconButton
            onClick={handleCopyText}
            disabled={!hasTranscription || retrying}
            title={t("settings.history.copyToClipboard")}
          >
            {showCopied ? (
              <Check width={16} height={16} />
            ) : (
              <Copy width={16} height={16} />
            )}
          </IconButton>
          <IconButton
            onClick={onToggleSaved}
            disabled={retrying}
            active={entry.saved}
            title={
              entry.saved
                ? t("settings.history.unsave")
                : t("settings.history.save")
            }
          >
            <Star
              width={16}
              height={16}
              fill={entry.saved ? "currentColor" : "none"}
            />
          </IconButton>
          <IconButton
            onClick={handleRetranscribe}
            disabled={retrying}
            title={t("settings.history.retranscribe")}
          >
            <RotateCcw
              width={16}
              height={16}
              style={
                retrying
                  ? { animation: "spin 1s linear infinite reverse" }
                  : undefined
              }
            />
          </IconButton>
          <IconButton
            onClick={handleDeleteEntry}
            disabled={retrying}
            title={t("settings.history.delete")}
          >
            <Trash2 width={16} height={16} />
          </IconButton>
        </div>
      </div>

      <p
        className={`italic text-sm pb-2 ${
          retrying
            ? ""
            : hasTranscription
              ? "text-text/90 select-text cursor-text whitespace-pre-wrap break-words"
              : "text-text/40"
        }`}
        style={
          retrying
            ? { animation: "transcribe-pulse 3s ease-in-out infinite" }
            : undefined
        }
      >
        {retrying && (
          <style>{`
            @keyframes transcribe-pulse {
              0%, 100% { color: color-mix(in srgb, var(--color-text) 40%, transparent); }
              50% { color: color-mix(in srgb, var(--color-text) 90%, transparent); }
            }
          `}</style>
        )}
        {retrying
          ? t("settings.history.transcribing")
          : hasTranscription
            ? entry.transcription_text
            : t("settings.history.transcriptionFailed")}
      </p>

      <CorrectionRowEditor entryId={entry.id} disabled={retrying} />

      <AudioPlayer onLoadRequest={handleLoadAudio} className="w-full" />
    </div>
  );
};
