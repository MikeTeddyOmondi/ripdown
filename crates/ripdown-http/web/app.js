// Minimal vanilla front-end — no framework, no build step.
const $ = (id) => document.getElementById(id);

async function submitDownload(e) {
  e.preventDefault();
  const url = $("url").value.trim();
  if (!url) return;
  const body = {
    url,
    format: $("format").value,
    audio_only: $("audio").checked,
  };
  $("url").value = "";
  try {
    await fetch("/api/download", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(body),
    });
  } catch (err) {
    console.error("download request failed", err);
  }
  refresh();
}

function statusLabel(status) {
  const s = status.state;
  if (s === "downloading") return "DOWNLOADING";
  if (s === "fetching_info") return "FETCHING";
  return s.toUpperCase();
}

function progressPct(status) {
  switch (status.state) {
    case "queued": return 0;
    case "fetching_info": return 5;
    case "downloading": return status.progress ?? 10;
    case "merging": return 98;
    case "done": return 100;
    default: return 0;
  }
}

function row(item) {
  const label = statusLabel(item.status);
  const pct = progressPct(item.status);
  const title = item.title || item.url;
  return `<tr>
    <td class="id">${item.id}</td>
    <td class="platform">${item.platform || "…"}</td>
    <td>${escapeHtml(title)}</td>
    <td><span class="status ${label}">${label}</span></td>
    <td><div class="bar"><span style="width:${pct}%"></span></div></td>
  </tr>`;
}

function escapeHtml(s) {
  return s.replace(/[&<>"']/g, (c) =>
    ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c])
  );
}

async function refresh() {
  try {
    const snap = await (await fetch("/api/queue")).json();
    const body = $("queue-body");
    if (snap.items.length === 0) {
      body.innerHTML =
        '<tr class="empty"><td colspan="5">No downloads yet. Paste a URL above.</td></tr>';
    } else {
      body.innerHTML = snap.items.map(row).join("");
    }
    $("s-queued").textContent = snap.queued;
    $("s-active").textContent = snap.active;
    $("s-done").textContent = snap.done;
    $("s-failed").textContent = snap.failed;

    const files = await (await fetch("/api/files")).json();
    $("files").innerHTML = files.length
      ? files
          .map(
            (f) =>
              `<li><a href="/api/files/${encodeURIComponent(f.key)}">${escapeHtml(
                f.key
              )}</a><span class="size">${fmtSize(f.size)}</span></li>`
          )
          .join("")
      : '<li style="color:var(--muted)">No files yet.</li>';
  } catch (err) {
    console.error("refresh failed", err);
  }
}

function fmtSize(n) {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  if (n < 1024 * 1024 * 1024) return `${(n / 1024 / 1024).toFixed(1)} MB`;
  return `${(n / 1024 / 1024 / 1024).toFixed(2)} GB`;
}

$("add-form").addEventListener("submit", submitDownload);
refresh();
setInterval(refresh, 1000);
