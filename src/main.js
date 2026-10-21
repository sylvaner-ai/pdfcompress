const { invoke } = window.__TAURI__.core;
const { getCurrentWebview } = window.__TAURI__.webview;

const PRESETS = {
  strong: { dpi: 100, quality: 50 },
  balanced: { dpi: 150, quality: 70 },
  light: { dpi: 220, quality: 85 },
};
const MAX_PARALLEL = 3;

const $ = (sel) => document.querySelector(sel);
const dropZone = $("#drop-zone");
const dropHint = $("#drop-hint");
const list = $("#files");
const summary = $("#summary");
const prefs = $("#prefs");
const dpi = $("#dpi");
const quality = $("#quality");

let settings = { preset: "balanced", imageDpi: 150, jpegQuality: 70 };
let engineReady = false;
const queue = [];
const inProgress = new Set();
let running = 0;
let totalBefore = 0;
let totalAfter = 0;

// ---------- Formatage ----------

function formatSize(bytes) {
  const units = ["o", "Ko", "Mo", "Go"];
  let i = 0;
  let v = bytes;
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024;
    i++;
  }
  return `${v.toLocaleString("fr-FR", { maximumFractionDigits: i === 0 ? 0 : 1 })} ${units[i]}`;
}

function basename(path) {
  return path.split(/[\\/]/).pop();
}

// ---------- Liste des fichiers ----------

function addRow(path) {
  const li = document.createElement("li");
  li.className = "file pending";
  li.title = path;
  li.innerHTML = `<span class="name"></span><span class="status"><div class="spinner"></div></span><span class="detail">En attente…</span>`;
  li.querySelector(".name").textContent = basename(path);
  list.prepend(li);
  document.body.classList.add("has-files");
  return li;
}

function updateSummary() {
  if (totalBefore === 0) return;
  const saved = totalBefore - totalAfter;
  const pct = Math.round((saved / totalBefore) * 100);
  summary.hidden = false;
  summary.textContent = `Gain total : ${formatSize(saved)} (−${pct} %)`;
}

async function processNext() {
  while (running < MAX_PARALLEL && queue.length > 0) {
    const { path, row } = queue.shift();
    running++;
    row.querySelector(".detail").textContent = "Compression…";
    invoke("compress_file", { path })
      .then((r) => {
        totalBefore += r.originalSize;
        totalAfter += r.newSize;
        const status = row.querySelector(".status");
        const detail = row.querySelector(".detail");
        if (r.replaced) {
          const pct = Math.round((1 - r.newSize / r.originalSize) * 100);
          row.className = "file done";
          status.textContent = pct > 0 ? `−${pct} %` : "−<1 %";
          detail.textContent = `${formatSize(r.originalSize)} → ${formatSize(r.newSize)}`;
        } else {
          row.className = "file same";
          status.textContent = "=";
          detail.textContent = `Déjà optimisé · ${formatSize(r.originalSize)}, fichier inchangé`;
        }
        updateSummary();
      })
      .catch((err) => {
        row.className = "file failed";
        row.querySelector(".status").textContent = "!";
        row.querySelector(".detail").textContent = String(err);
      })
      .finally(() => {
        running--;
        inProgress.delete(path);
        processNext();
      });
  }
}

async function handleDrop(paths) {
  if (!engineReady) {
    openPrefs();
    return;
  }
  const pdfs = await invoke("collect_pdfs", { paths });
  if (pdfs.length === 0) {
    dropZone.classList.add("error");
    dropHint.textContent = "Aucun fichier PDF dans la sélection";
    setTimeout(() => {
      dropZone.classList.remove("error");
      dropHint.textContent = "Fichiers ou dossiers · compression sur place";
    }, 2500);
    return;
  }
  for (const path of pdfs) {
    if (inProgress.has(path)) continue;
    inProgress.add(path);
    queue.push({ path, row: addRow(path) });
  }
  processNext();
}

getCurrentWebview().onDragDropEvent((event) => {
  const { type } = event.payload;
  if (type === "enter" || type === "over") {
    dropZone.classList.add("active");
  } else if (type === "drop") {
    dropZone.classList.remove("active");
    handleDrop(event.payload.paths);
  } else {
    dropZone.classList.remove("active");
  }
});

// ---------- Préférences ----------

function renderSettings() {
  document.querySelectorAll('input[name="preset"]').forEach((r) => {
    r.checked = r.value === settings.preset;
  });
  const values = PRESETS[settings.preset] ?? { dpi: settings.imageDpi, quality: settings.jpegQuality };
  dpi.value = values.dpi;
  quality.value = values.quality;
  $("#dpi-out").textContent = `${values.dpi} dpi`;
  $("#quality-out").textContent = `${values.quality} %`;
}

async function saveSettings() {
  renderSettings();
  try {
    await invoke("save_settings", { settings });
  } catch (err) {
    console.error(err);
  }
}

document.querySelectorAll('input[name="preset"]').forEach((radio) => {
  radio.addEventListener("change", () => {
    settings.preset = radio.value;
    saveSettings();
  });
});

// Déplacer un curseur bascule en mode personnalisé, en partant des valeurs affichées.
for (const input of [dpi, quality]) {
  input.addEventListener("input", () => {
    settings.preset = "custom";
    settings.imageDpi = Number(dpi.value);
    settings.jpegQuality = Number(quality.value);
    renderSettings();
  });
  input.addEventListener("change", saveSettings);
}

function openPrefs() {
  prefs.hidden = false;
}
function closePrefs() {
  prefs.hidden = true;
}

$("#open-prefs").addEventListener("click", openPrefs);
$("#close-prefs").addEventListener("click", closePrefs);
prefs.addEventListener("click", (e) => {
  if (e.target === prefs) closePrefs();
});
document.addEventListener("keydown", (e) => {
  if (e.metaKey && e.key === ",") {
    e.preventDefault();
    openPrefs();
  } else if (e.key === "Escape") {
    closePrefs();
  }
});

// ---------- Démarrage ----------

async function init() {
  settings = await invoke("get_settings");
  renderSettings();

  const engine = await invoke("engine_info");
  const engineLine = $("#engine");
  engineReady = engine.available;
  if (engine.available) {
    engineLine.textContent = `Moteur : Ghostscript ${engine.version ?? ""} ${engine.bundled ? "(intégré)" : "(installé sur le Mac)"}`;
  } else {
    engineLine.classList.add("missing");
    engineLine.textContent = "Ghostscript est introuvable. Installez-le avec « brew install ghostscript ».";
    dropHint.textContent = "Moteur de compression introuvable";
    dropZone.classList.add("error");
  }
}

init();
