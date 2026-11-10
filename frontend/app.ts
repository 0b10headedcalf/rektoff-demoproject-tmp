import { basicSetup, EditorView } from "codemirror";
import { rust } from "@codemirror/lang-rust";
import { oneDark } from "@codemirror/theme-one-dark";

const LEVEL = "level-01-missing-signer";

type Stage = "queued" | "building" | "testing" | "done";

interface JobState {
  stage: Stage;
  passed: boolean | null;
  checks: { name: string; passed: boolean }[];
  compile_error: string | null;
  error: string | null;
  queue_ms: number | null;
  build_ms: number | null;
  test_ms: number | null;
}

const STAGE_TEXT: Record<Stage, string> = {
  queued: "Waiting in the queue…",
  building: "Compiling your program…",
  testing: "Attacking your fix…",
  done: "",
};

function el<T extends HTMLElement>(id: string): T {
  const node = document.getElementById(id);
  if (!node) throw new Error(`missing #${id}`);
  return node as T;
}

const descriptionEl = el<HTMLDivElement>("description");
const submitButton = el<HTMLButtonElement>("submit");
const resetButton = el<HTMLButtonElement>("reset");
const statusEl = el<HTMLDivElement>("status");
const checksEl = el<HTMLUListElement>("checks");
const compileErrorEl = el<HTMLPreElement>("compile-error");
const timingsEl = el<HTMLDivElement>("timings");

let editor: EditorView | undefined;
let originalSource = "";

/** Renders the level's small Markdown subset: a heading, paragraphs, **bold**. */
function renderDescription(markdown: string) {
  descriptionEl.replaceChildren();
  for (const block of markdown.trim().split(/\n\s*\n/)) {
    const heading = block.startsWith("# ");
    const node = document.createElement(heading ? "h1" : "p");
    const text = (heading ? block.slice(2) : block).replace(/\n/g, " ");
    for (const [i, part] of text.split("**").entries()) {
      const child = i % 2 ? document.createElement("strong") : document.createTextNode(part);
      if (child instanceof HTMLElement) child.textContent = part;
      node.append(child);
    }
    descriptionEl.append(node);
  }
}

function setSource(source: string) {
  editor?.dispatch({ changes: { from: 0, to: editor.state.doc.length, insert: source } });
}

function clearResults() {
  checksEl.replaceChildren();
  compileErrorEl.hidden = true;
  timingsEl.textContent = "";
  statusEl.className = "status";
}

function showStatus(text: string, kind: "" | "pass" | "fail" | "busy" = "") {
  statusEl.textContent = text;
  statusEl.className = `status ${kind}`.trim();
}

function showResult(state: JobState) {
  if (state.error) {
    showStatus(state.error, "fail");
  } else if (state.compile_error) {
    showStatus("Your program did not compile.", "fail");
    compileErrorEl.textContent = state.compile_error;
    compileErrorEl.hidden = false;
  } else if (state.passed) {
    showStatus("All checks passed. The exploit no longer works.", "pass");
  } else {
    showStatus("Not yet. Some checks failed:", "fail");
  }

  for (const check of state.checks) {
    const item = document.createElement("li");
    item.className = check.passed ? "pass" : "fail";
    item.textContent = `${check.passed ? "✓" : "✗"} ${check.name}`;
    checksEl.append(item);
  }

  const stages = [
    ["queue", state.queue_ms],
    ["build", state.build_ms],
    ["test", state.test_ms],
  ].filter(([, ms]) => ms != null);
  timingsEl.textContent = stages.map(([name, ms]) => `${name} ${ms} ms`).join(" · ");
}

async function submit() {
  if (!editor) return;
  submitButton.disabled = true;
  clearResults();
  showStatus("Submitting…", "busy");

  const finish = () => (submitButton.disabled = false);
  try {
    const response = await fetch("/api/submit", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ level: LEVEL, source: editor.state.doc.toString() }),
    });
    if (!response.ok) {
      showStatus(`Could not submit: ${await response.text()}`, "fail");
      return finish();
    }
    const { id } = (await response.json()) as { id: string };

    const events = new EventSource(`/api/submit/${id}/events`);
    events.addEventListener("status", (event) => {
      const state = JSON.parse((event as MessageEvent).data) as JobState;
      showStatus(STAGE_TEXT[state.stage], "busy");
    });
    events.addEventListener("done", (event) => {
      events.close();
      showResult(JSON.parse((event as MessageEvent).data) as JobState);
      finish();
    });
    events.onerror = async () => {
      // The stream dropped: fall back to asking once for the current state.
      events.close();
      const latest = await fetch(`/api/submit/${id}`).catch(() => undefined);
      const state = latest?.ok ? ((await latest.json()) as JobState) : undefined;
      if (state?.stage === "done") showResult(state);
      else showStatus("Lost connection to the server. Please submit again.", "fail");
      finish();
    };
  } catch {
    showStatus("Could not reach the server.", "fail");
    finish();
  }
}

async function load() {
  const response = await fetch(`/api/levels/${LEVEL}`).catch(() => undefined);
  if (!response?.ok) {
    showStatus("Could not load the level. Is the backend running?", "fail");
    return;
  }
  const level = (await response.json()) as { description: string; source: string };
  originalSource = level.source;
  renderDescription(level.description);

  editor = new EditorView({
    doc: level.source,
    extensions: [basicSetup, rust(), oneDark],
    parent: el("editor"),
  });
  submitButton.disabled = false;
  resetButton.disabled = false;
}

submitButton.addEventListener("click", submit);
resetButton.addEventListener("click", () => {
  setSource(originalSource);
  clearResults();
  showStatus("");
});

load();
