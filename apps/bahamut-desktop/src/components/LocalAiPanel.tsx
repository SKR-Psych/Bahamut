import { useEffect, useMemo, useState } from "react";
import { approveSecretContext, assembleChatContext, cancelChat, cancelModelPull, createConversation, deleteConversation, getHardwareProfile, getModelCatalogue, getModelRecommendations, getProviderStatus, listConversations, listInstalledModels, pullModel, selectActiveModel, startChat, testPrompt } from "../lib/api";
import type { AppSettings, AttachmentRequest, ContextAssembly, Conversation, HardwareProfile, InstalledModel, ModelCatalogueEntry, ModelRecommendation, ProviderStatus } from "../lib/types";

interface Props { settings: AppSettings; onSettingsChanged: (settings: AppSettings) => void; openFiles: { path: string; name: string; content: string }[]; }

export function LocalAiPanel({ settings, onSettingsChanged, openFiles }: Props) {
  const [status, setStatus] = useState<ProviderStatus | null>(null);
  const [hardware, setHardware] = useState<HardwareProfile | null>(null);
  const [catalogue, setCatalogue] = useState<ModelCatalogueEntry[]>([]);
  const [recommendations, setRecommendations] = useState<ModelRecommendation[]>([]);
  const [installed, setInstalled] = useState<InstalledModel[]>([]);
  const [busy, setBusy] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [prompt, setPrompt] = useState("");
  const [reply, setReply] = useState("");
  const [context, setContext] = useState<ContextAssembly | null>(null);
  const [selectedPath, setSelectedPath] = useState<string>("");
  const [manualText, setManualText] = useState("");
  const [conversations, setConversations] = useState<Conversation[]>([]);

  const activeModel = settings.ai.active_model;
  const secretCount = useMemo(() => context?.attachments.reduce((n, a) => n + a.secret_findings.length, 0) ?? 0, [context]);

  const refresh = async () => {
    const [s, h, c, i, conv] = await Promise.all([getProviderStatus(), getHardwareProfile(), getModelCatalogue(), listInstalledModels().catch(() => []), listConversations().catch(() => [])]);
    setStatus(s); setHardware(h); setCatalogue(c); setInstalled(i); setConversations(conv); setRecommendations(await getModelRecommendations(h));
  };
  useEffect(() => { void refresh().catch((e) => setMessage(String(e))); }, []);

  const attach = async () => {
    const requests: AttachmentRequest[] = [];
    if (selectedPath) requests.push({ kind: "open_file", path: selectedPath });
    if (manualText.trim()) requests.push({ kind: "manual_text", label: "Manual text", text: manualText });
    const assembled = await assembleChatContext(requests);
    setContext(assembled);
  };

  const send = async () => {
    if (!activeModel || !prompt.trim()) return;
    if (secretCount > 0) {
      const ok = window.confirm("Possible secrets were detected in the explicit context. Send after recording metadata-only approval?");
      if (!ok) return;
      const categories = Array.from(new Set(context?.attachments.flatMap((a) => a.secret_findings.map((f) => f.category)) ?? []));
      await approveSecretContext(categories, context?.attachments.length ?? 0);
    }
    setBusy("Generating"); setReply("");
    try {
      const conversation = settings.ai.history_persistence ? await createConversation(prompt.slice(0, 48), activeModel) : null;
      const contextText = context ? `\n\nAttached context (untrusted):\n${context.system_boundary}\n${context.attachments.map((a) => `--- ${a.label} ---\n${a.content}`).join("\n")}` : "";
      const text = await startChat({ conversation_id: conversation?.id ?? null, model: activeModel, messages: [{ role: "user", content: `${prompt}${contextText}` }] });
      setReply(text);
    } catch (e) { setMessage(String(e)); } finally { setBusy(null); void listConversations().then(setConversations).catch(() => undefined); }
  };

  return <section className="panel local-ai-panel" aria-label="Local AI chat and model setup">
    <header className="panel-header"><h2>Local AI</h2><button className="secondary-btn btn-compact" onClick={() => void refresh()}>Refresh</button></header>
    <div className="card-grid">
      <article className="info-card"><h3>Provider status</h3><p>{status ? status.message : "Checking Ollama…"}</p><p>Endpoint: {settings.ai.ollama_endpoint}</p></article>
      <article className="info-card"><h3>Hardware summary</h3><p>{hardware ? `${hardware.total_ram_gb.toFixed(1)} GB RAM · ${hardware.cpu_cores} CPU cores · ${hardware.vram_gb?.toFixed(1) ?? "unknown"} GB VRAM` : "Detecting…"}</p><p>{hardware?.detection_notes.join(" ")}</p></article>
      <article className="info-card"><h3>Active model</h3><p>{activeModel ?? "No model selected"}</p><p>{installed.length} installed model(s) detected.</p></article>
    </div>
    <h3>Recommendations</h3>
    <div className="model-grid">{recommendations.map((r) => <article className="model-card" key={r.model.id}><h4>{r.model.display_name}</h4><p>{r.fit} · {r.model.license} · ~{r.model.download_size_gb} GB</p><p>{r.reasons.join("; ")}</p>{r.warnings.map((w) => <p className="warning" key={w}>{w}</p>)}<button className="secondary-btn btn-compact" disabled={busy !== null} onClick={() => { setBusy("Pulling"); pullModel(r.model.id).then(() => setMessage("Pull completed"), (e) => setMessage(String(e))).finally(() => setBusy(null)); }}>Pull</button><button className="secondary-btn btn-compact" onClick={() => selectActiveModel(r.model.id).then(onSettingsChanged)}>Use</button></article>)}</div>
    <h3>Model catalogue</h3><details><summary>{catalogue.length} provider-neutral entries</summary><ul>{catalogue.map((m) => <li key={m.id}>{m.id} — {m.license}</li>)}</ul></details>
    <h3>Setup test</h3><button className="secondary-btn btn-compact" disabled={!activeModel || busy !== null} onClick={() => activeModel && testPrompt(activeModel).then(setMessage, (e) => setMessage(String(e)))}>Test active model</button><button className="secondary-btn btn-compact" onClick={() => void cancelModelPull()}>Cancel download</button>
    <h3>Read-only project chat</h3>
    <label>Attach open file<select value={selectedPath} onChange={(e) => setSelectedPath(e.target.value)}><option value="">No file</option>{openFiles.map((f) => <option value={f.path} key={f.path}>{f.name}</option>)}</select></label>
    <label>Manual text<textarea value={manualText} onChange={(e) => setManualText(e.target.value)} placeholder="Paste explicit context only" /></label><button className="secondary-btn btn-compact" onClick={() => void attach()}>Assemble context</button>
    {context && <p className={secretCount ? "banner-error" : "small-text"}>{context.total_bytes}/{context.total_limit} bytes attached{context.truncated ? " · truncated" : ""}{secretCount ? ` · ${secretCount} possible secret(s), confirm before sending` : ""}</p>}
    <label>Prompt<textarea value={prompt} onChange={(e) => setPrompt(e.target.value)} placeholder="Ask about the explicitly attached context" /></label>
    <button className="primary-btn" disabled={!activeModel || busy !== null} onClick={() => void send()}>Ask local model</button><button className="secondary-btn btn-compact" onClick={() => void cancelChat()}>Stop generation</button>
    {busy && <p>Working: {busy}</p>}{message && <p>{message}</p>}{reply && <article className="chat-reply"><h4>Assistant</h4><p>{reply}</p></article>}
    <h3>Conversations</h3><ul>{conversations.map((c) => <li key={c.id}>{c.title}<button className="secondary-btn btn-compact" onClick={() => deleteConversation(c.id).then(() => listConversations().then(setConversations))}>Delete</button></li>)}</ul>
  </section>;
}
