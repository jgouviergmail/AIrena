import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { BookOpen, Database, FileText, Globe, Info, Loader2, Plus, Upload, X } from "lucide-react";
import { TokenBudgetPreviewPanel } from "@/components/setup/TokenBudgetPreview";
import { useSetupStore } from "@/stores/useSetupStore";
import { useSettingsStore } from "@/stores/useSettingsStore";
import type { BudgetParams, SectionPriority, TokenBudgetPreview } from "@/lib/types";
import * as api from "@/lib/tauri-api";
import { extractErrorMessage } from "@/lib/error-utils";
import { toast } from "@/stores/useToastStore";
import { modeSupportsHiddenAgenda } from "@/lib/modes";
import { inputClass, OptionCard, RAG_SUPPORTED_EXTENSIONS, SectionLabel } from "./shared";

export function StepKnowledge() {
  const { t } = useTranslation();
  const ragDocuments = useSetupStore((s) => s.ragDocuments);
  const addRagDocument = useSetupStore((s) => s.addRagDocument);
  const removeRagDocument = useSetupStore((s) => s.removeRagDocument);
  const gladiateurs = useSetupStore((s) => s.gladiateurs);
  const maxTurns = useSetupStore((s) => s.maxTurns);
  const webSearchPool = useSetupStore((s) => s.webSearchPool);
  const setWebSearchPool = useSetupStore((s) => s.setWebSearchPool);
  const wikiSearchPool = useSetupStore((s) => s.wikiSearchPool);
  const setWikiSearchPool = useSetupStore((s) => s.setWikiSearchPool);
  const arbitre = useSetupStore((s) => s.arbitre);
  const discussionLanguage = useSetupStore((s) => s.discussionLanguage);
  const tokenBudgetPreview = useSetupStore((s) => s.tokenBudgetPreview);
  const setTokenBudgetPreview = useSetupStore((s) => s.setTokenBudgetPreview);
  const documentInjectionMode = useSetupStore((s) => s.documentInjectionMode);
  const features = useSetupStore((s) => s.features);
  const discussionMode = useSetupStore((s) => s.discussionMode);
  const engineConstants = useSettingsStore((s) => s.engineConstants);
  const loadEngineConstants = useSettingsStore((s) => s.loadEngineConstants);
  const setDocumentInjectionMode = useSetupStore((s) => s.setDocumentInjectionMode);
  const settingsNumCtx = useSettingsStore((s) => s.settings.numCtx);
  const provider = useSettingsStore((s) => s.settings.llmProvider);
  const ollamaModel = useSettingsStore((s) => s.settings.ollamaModel);
  const embeddingModel = useSettingsStore((s) => s.settings.embeddingModel);
  const hasTavilyKey = !!useSettingsStore((s) => s.settings.tavilyApiKey).trim();
  const tokenBudgetPriorities = useSettingsStore((s) => s.settings.tokenBudgetPriorities);
  const [ragImporting, setRagImporting] = useState(false);
  const [dragOver, setDragOver] = useState(false);

  const maxSearchBound = (maxTurns ?? 100) * Math.max(gladiateurs.length, 1);

  // Semantic search needs an Ollama embedding model; a cloud provider without one
  // still gets lexical (BM25) retrieval — documents are imported without embeddings.
  const embeddingsAvailable = provider === "ollama" ? !!(embeddingModel || ollamaModel) : !!embeddingModel;
  const skipEmbeddings = documentInjectionMode === "fullInjection" || !embeddingsAvailable;

  const importFiles = async (paths: string[]) => {
    setRagImporting(true);
    try {
      for (const filePath of paths) {
        try {
          const doc = await api.importRagDocument(filePath, skipEmbeddings);
          addRagDocument(doc);
        } catch (e: unknown) {
          const detail = extractErrorMessage(e);
          const fileName = filePath.split(/[\\/]/).pop() ?? filePath;
          toast.error(t("setup.ragImportError", { file: fileName }), detail);
        }
      }
    } finally {
      setRagImporting(false);
    }
  };

  const handleRagImport = async () => {
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const selected = await open({
        multiple: true,
        filters: [{ name: "Documents", extensions: [...RAG_SUPPORTED_EXTENSIONS] }],
      });
      if (!selected || selected.length === 0) return;
      const paths = Array.isArray(selected) ? selected : [selected];
      await importFiles(paths);
    } catch (e: unknown) {
      toast.error(t("setup.ragImportError", { file: "" }), extractErrorMessage(e));
    }
  };

  // Ref to always call the latest drop handler without re-subscribing
  const dropHandlerRef = useRef<(paths: string[]) => void>(() => {});
  dropHandlerRef.current = (paths: string[]) => {
    const validPaths = paths.filter((p) => {
      const ext = p.split(".").pop()?.toLowerCase() ?? "";
      return RAG_SUPPORTED_EXTENSIONS.has(ext);
    });
    if (validPaths.length === 0) {
      toast.warning(t("setup.ragUnsupportedFormat"));
      return;
    }
    importFiles(validPaths);
  };

  // Subscribe to Tauri drag-drop events (global webview-level)
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let mounted = true;
    (async () => {
      const { getCurrentWebviewWindow } = await import("@tauri-apps/api/webviewWindow");
      const unsub = await getCurrentWebviewWindow().onDragDropEvent((event) => {
        if (!mounted) return;
        if (event.payload.type === "enter" || event.payload.type === "over") {
          setDragOver(true);
        } else if (event.payload.type === "leave") {
          setDragOver(false);
        } else if (event.payload.type === "drop") {
          setDragOver(false);
          dropHandlerRef.current(event.payload.paths);
        }
      });
      if (mounted) {
        unlisten = unsub;
      } else {
        unsub();
      }
    })();
    return () => {
      mounted = false;
      unlisten?.();
    };
  }, []);

  // The agenda block size comes from the backend (cached once loaded)
  useEffect(() => {
    if (features.hiddenAgenda) loadEngineConstants();
  }, [features.hiddenAgenda, loadEngineConstants]);

  // Compute token budget preview when relevant params change
  const computeBudget = useCallback(async (): Promise<TokenBudgetPreview> => {
    const totalDocChars = ragDocuments.reduce((sum, d) => sum + d.charCount, 0);
    const params: BudgetParams = {
      numCtx: settingsNumCtx,
      numPredict: arbitre.llmParams.numPredict,
      systemPromptChars: arbitre.systemPrompt.length,
      nGladiateurs: gladiateurs.length || 1,
      language: discussionLanguage,
      features: {
        webSearchEnabled: webSearchPool > 0 || (arbitre.webSearchIntro ?? false),
        wikiSearchEnabled: wikiSearchPool > 0 || (arbitre.wikiSearchIntro ?? false),
        ragEnabled: ragDocuments.length > 0,
        documentChars: ragDocuments.length > 0 && documentInjectionMode === "fullInjection" ? totalDocChars : 0,
        agendaChars: features.hiddenAgenda && modeSupportsHiddenAgenda(discussionMode) ? (engineConstants?.agendaBlockMaxChars ?? 0) : 0,
      },
      provider,
    };

    let priorities: SectionPriority[] = [];
    try {
      if (tokenBudgetPriorities) priorities = JSON.parse(tokenBudgetPriorities);
    } catch { /* use defaults on backend */ }

    return api.computeTokenBudget(params, priorities);
  }, [
    settingsNumCtx, arbitre.llmParams.numPredict, arbitre.systemPrompt.length,
    gladiateurs.length, discussionLanguage, webSearchPool, wikiSearchPool,
    ragDocuments, arbitre.webSearchIntro, arbitre.wikiSearchIntro,
    tokenBudgetPriorities, documentInjectionMode, provider,
    features.hiddenAgenda, discussionMode, engineConstants?.agendaBlockMaxChars,
  ]);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const preview = await computeBudget();
        if (!cancelled) setTokenBudgetPreview(preview);
      } catch {
        if (!cancelled) setTokenBudgetPreview(null);
      }
    })();
    return () => { cancelled = true; };
  }, [computeBudget, setTokenBudgetPreview]);

  return (
    <div className="space-y-6">
      {/* Token Budget Preview — always visible at the top */}
      <TokenBudgetPreviewPanel
        preview={tokenBudgetPreview}
        documentInjectionMode={documentInjectionMode}
        hasDocuments={ragDocuments.length > 0}
        nGladiateurs={gladiateurs.length || 1}
        numPredict={arbitre.llmParams.numPredict}
      />

      <p className="text-sm text-muted-foreground">{t("setup.knowledgeStepDesc")}</p>

      {/* RAG Knowledge Base */}
      <div className="space-y-2">
        <SectionLabel icon={Database} iconClassName="text-purple-500">{t("setup.ragKnowledgeBase")}</SectionLabel>
        <p className="text-xs text-muted-foreground">{t("setup.ragDesc")}</p>

        {provider === "ollama" && !embeddingModel && ollamaModel && (
          <InfoNote>{t("setup.ragRecommendModel")}</InfoNote>
        )}
        {provider === "deepseek" && !embeddingModel && (
          <InfoNote>{t("setup.ragLexicalOnly")}</InfoNote>
        )}

        {dragOver && (
          <div className="flex flex-col items-center justify-center rounded-md border-2 border-dashed border-purple-500/50 bg-purple-500/5 py-6">
            <Upload className="h-8 w-8 text-purple-500" />
            <p className="mt-2 text-sm font-medium text-purple-500">{t("setup.ragDropFiles")}</p>
          </div>
        )}

        <div className="flex flex-wrap items-center gap-2">
          <button
            onClick={handleRagImport}
            disabled={ragImporting}
            className="flex items-center gap-1.5 rounded-md border border-border px-3 py-1.5 text-sm text-foreground transition-colors hover:bg-accent disabled:opacity-50"
          >
            {ragImporting ? <Loader2 className="h-3.5 w-3.5 animate-spin" /> : <Plus className="h-3.5 w-3.5" />}
            {ragImporting ? t("setup.ragImporting") : t("setup.ragImportFiles")}
          </button>
          <span className="text-xs text-muted-foreground">{t("setup.ragDropHint")}</span>
        </div>

        {ragDocuments.length > 0 && (
          <div className="space-y-1.5">
            {ragDocuments.map((doc) => (
              <div
                key={doc.docId}
                className="flex items-center justify-between gap-2 rounded-md border border-border bg-card px-3 py-2"
              >
                <div className="flex min-w-0 flex-wrap items-center gap-2 text-sm">
                  <Database className="h-3.5 w-3.5 shrink-0 text-purple-500" />
                  <span className="truncate font-medium text-foreground">{doc.fileName}</span>
                  <span className="text-xs text-muted-foreground">
                    .{doc.format} — {doc.chunkCount} {t("setup.ragChunks")}, {doc.charCount.toLocaleString()} {t("setup.ragChars")}
                  </span>
                </div>
                <button
                  onClick={async () => {
                    try {
                      await api.removeRagDocument(doc.docId);
                      removeRagDocument(doc.docId);
                    } catch (e: unknown) {
                      toast.error(t("setup.ragRemoveError"), extractErrorMessage(e));
                    }
                  }}
                  className="shrink-0 rounded p-1 text-muted-foreground hover:bg-destructive/10 hover:text-destructive"
                >
                  <X className="h-3.5 w-3.5" />
                </button>
              </div>
            ))}
          </div>
        )}
      </div>

      {/* Document injection mode choice — only when documents are imported */}
      {ragDocuments.length > 0 && (
        <div className="space-y-2">
          <SectionLabel icon={FileText}>{t("setup.injectionModeLabel")}</SectionLabel>
          <div className="grid grid-cols-1 gap-2 sm:grid-cols-2">
            <OptionCard
              selected={documentInjectionMode === "rag"}
              title={t("setup.injectionModeRag")}
              description={t("setup.injectionModeRagDesc")}
              onClick={() => setDocumentInjectionMode("rag")}
            />
            <OptionCard
              selected={documentInjectionMode === "fullInjection"}
              title={t("setup.injectionModeFull")}
              description={t("setup.injectionModeFullDesc")}
              onClick={() => setDocumentInjectionMode("fullInjection")}
            />
          </div>
        </div>
      )}

      {/* Web search pool */}
      {hasTavilyKey ? (
        <div className="space-y-2">
          <SectionLabel icon={Globe}>{t("setup.webSearchPool")}</SectionLabel>
          <div className="flex flex-wrap items-center gap-3">
            <input
              type="number"
              min={0}
              max={maxSearchBound}
              value={webSearchPool}
              onChange={(e) => setWebSearchPool(Math.max(0, Math.min(maxSearchBound, parseInt(e.target.value) || 0)))}
              className={`${inputClass} w-24`}
            />
            <span className="text-sm text-muted-foreground">{t("setup.webSearchPoolDesc", { max: maxSearchBound })}</span>
          </div>
          {webSearchPool > 0 && (
            <p className="text-xs text-muted-foreground">{t("setup.webSearchBudget", { count: webSearchPool })}</p>
          )}
        </div>
      ) : (
        <div className="flex items-center gap-2 rounded-md border border-dashed border-border px-3 py-2 text-xs text-muted-foreground">
          <Globe className="h-3.5 w-3.5" />
          {t("setup.webSearchNoKey")}
        </div>
      )}

      {/* Wiki search pool */}
      <div className="space-y-2">
        <SectionLabel icon={BookOpen} iconClassName="text-green-600">{t("setup.wikiSearchPool")}</SectionLabel>
        <div className="flex flex-wrap items-center gap-3">
          <input
            type="number"
            min={0}
            max={maxSearchBound}
            value={wikiSearchPool}
            onChange={(e) => setWikiSearchPool(Math.max(0, Math.min(maxSearchBound, parseInt(e.target.value) || 0)))}
            className={`${inputClass} w-24`}
          />
          <span className="text-sm text-muted-foreground">{t("setup.wikiSearchPoolDesc", { max: maxSearchBound })}</span>
        </div>
        {wikiSearchPool > 0 && (
          <p className="text-xs text-muted-foreground">{t("setup.wikiSearchBudget", { count: wikiSearchPool })}</p>
        )}
      </div>

      {/* First-turn search rule explanation */}
      {(webSearchPool > 0 || wikiSearchPool > 0) && <InfoNote>{t("setup.firstTurnSearchRule")}</InfoNote>}
    </div>
  );
}

function InfoNote({ children }: { children: React.ReactNode }) {
  return (
    <div className="flex items-start gap-2 rounded-md border border-primary/20 bg-primary/5 px-3 py-2 text-xs text-muted-foreground">
      <Info className="mt-0.5 h-3.5 w-3.5 shrink-0 text-primary" />
      <span>{children}</span>
    </div>
  );
}
