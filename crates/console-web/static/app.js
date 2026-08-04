(() => {
  const transcript = document.getElementById("transcript");
  const rail = document.getElementById("rail");
  const railList = document.getElementById("railList");
  const composer = document.getElementById("composer");
  const promptEl = document.getElementById("prompt");
  const btnAsk = document.getElementById("btnAsk");
  const btnNew = document.getElementById("btnNew");
  const orb = document.getElementById("orb");
  const presenceLabel = document.getElementById("presenceLabel");
  const decision = document.getElementById("decision");
  const decisionSummary = document.getElementById("decisionSummary");
  const decisionMeta = document.getElementById("decisionMeta");

  let busy = false;
  let pending = null;
  let lastSummaryShown = "";

  function setPresence(kind, label) {
    orb.className = "orb" + (kind ? ` ${kind}` : "");
    presenceLabel.textContent = label;
  }

  function addBeat(role, text, cls = "") {
    if (!text) return;
    const article = document.createElement("article");
    article.className = `beat ${cls}`.trim();
    const roleEl = document.createElement("p");
    roleEl.className = "beat-role";
    roleEl.textContent = role;
    const body = document.createElement("p");
    body.textContent = text;
    article.append(roleEl, body);
    transcript.appendChild(article);
    article.scrollIntoView({ behavior: "smooth", block: "nearest" });
  }

  function pushRail(line) {
    rail.hidden = false;
    const li = document.createElement("li");
    li.textContent = line;
    railList.appendChild(li);
    while (railList.children.length > 12) {
      railList.removeChild(railList.firstChild);
    }
  }

  function showDecision(corr, p) {
    pending = { correlation_id: corr, ...p };
    decisionSummary.textContent = p.summary || `${p.tool}`;
    decisionMeta.textContent = `${p.tool} · ${JSON.stringify(p.arguments || {})}`;
    decision.hidden = false;
  }

  function hideDecision() {
    pending = null;
    decision.hidden = true;
  }

  async function refreshStatus() {
    try {
      const res = await fetch("/api/status");
      const data = await res.json();
      if (!res.ok || data.ok === false) {
        setPresence("bad", data.error || "offline");
        return;
      }
      const st = data.status || {};
      const provider = st.provider || "?";
      const cpuHint = st.telemetry ? ` · tel ${st.telemetry_samples || 0}` : "";
      setPresence(busy ? "busy" : "ok", `${provider}${cpuHint}`);
    } catch {
      setPresence("bad", "offline");
    }
  }

  async function ask(text) {
    if (busy || !text.trim()) return;
    busy = true;
    btnAsk.disabled = true;
    setPresence("busy", "думает…");
    addBeat("Вы", text.trim(), "user");
    promptEl.value = "";
    lastSummaryShown = "";

    try {
      const res = await fetch("/api/diagnose", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ text: text.trim() }),
      });
      if (!res.ok || !res.body) {
        const err = await res.json().catch(() => ({}));
        addBeat("Ошибка", err.error || res.statusText, "tool");
        return;
      }

      const reader = res.body.getReader();
      const decoder = new TextDecoder();
      let buffer = "";
      while (true) {
        const { value, done } = await reader.read();
        if (done) break;
        buffer += decoder.decode(value, { stream: true });
        let idx;
        while ((idx = buffer.indexOf("\n")) >= 0) {
          const line = buffer.slice(0, idx).trim();
          buffer = buffer.slice(idx + 1);
          if (!line) continue;
          handleFrame(JSON.parse(line));
        }
      }
      if (buffer.trim()) {
        handleFrame(JSON.parse(buffer.trim()));
      }
    } catch (e) {
      addBeat("Ошибка", String(e), "tool");
    } finally {
      busy = false;
      btnAsk.disabled = false;
      refreshStatus();
    }
  }

  function handleFrame(frame) {
    if (frame.type === "progress" && frame.event) {
      const ev = frame.event;
      const kind = ev.type;
      if (kind === "tool_call") {
        pushRail(`tool → ${ev.tool}`);
        addBeat("Шаг", `Вызов ${ev.tool}`, "tool");
      } else if (kind === "tool_result") {
        pushRail(`${ev.tool} → ${ev.ok ? "ok" : "fail"}`);
      } else if (kind === "policy") {
        pushRail(`policy ${ev.tool}=${ev.verdict}`);
      } else if (kind === "assistant" && ev.text) {
        lastSummaryShown = ev.text;
        addBeat("SaaiOS", ev.text);
      }
      return;
    }

    if (frame.type === "done") {
      if (frame.error) {
        addBeat("Ошибка", frame.error, "tool");
        return;
      }
      const summary = frame.diagnose && frame.diagnose.summary;
      if (summary && summary !== lastSummaryShown) {
        addBeat("SaaiOS", summary);
      }
      if (frame.pending && frame.correlation_id) {
        showDecision(frame.correlation_id, frame.pending);
      }
    }
  }

  async function confirm(scope) {
    if (!pending) return;
    const body = {
      correlation_id: pending.correlation_id,
      call_id: pending.call_id,
      tool: pending.tool,
      arguments: pending.arguments,
      scope,
    };
    hideDecision();
    setPresence("busy", "подтверждение…");
    try {
      const res = await fetch("/api/confirm", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify(body),
      });
      const data = await res.json();
      if (data.error) {
        addBeat("Ошибка", data.error, "tool");
      } else if (data.tool_result) {
        const tr = data.tool_result;
        addBeat(
          "Результат",
          `${tr.tool}: ${tr.ok ? "выполнено" : "не выполнено"}`,
          "tool"
        );
        pushRail(`confirm ${scope} → ${tr.ok ? "ok" : "fail"}`);
      } else {
        addBeat("Результат", `scope=${scope}`, "tool");
      }
    } catch (e) {
      addBeat("Ошибка", String(e), "tool");
    } finally {
      refreshStatus();
    }
  }

  composer.addEventListener("submit", (e) => {
    e.preventDefault();
    ask(promptEl.value);
  });

  promptEl.addEventListener("keydown", (e) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      ask(promptEl.value);
    }
  });

  btnNew.addEventListener("click", async () => {
    await fetch("/api/chat/reset", { method: "POST" });
    railList.innerHTML = "";
    rail.hidden = true;
    hideDecision();
    addBeat("Сцена", "Новый разговор. История сброшена.", "intro");
  });

  document.querySelectorAll("[data-prompt]").forEach((el) => {
    el.addEventListener("click", () => ask(el.getAttribute("data-prompt")));
  });

  decision.querySelectorAll("[data-scope]").forEach((el) => {
    el.addEventListener("click", () => confirm(el.getAttribute("data-scope")));
  });

  refreshStatus();
  setInterval(refreshStatus, 8000);

  if (document.body.classList.contains("kiosk")) {
    promptEl.focus();
    // Soft idle: keep prompt focused after decision closes / asks finish.
    document.addEventListener("click", () => {
      if (!busy && decision.hidden) promptEl.focus();
    });
  }
})();
