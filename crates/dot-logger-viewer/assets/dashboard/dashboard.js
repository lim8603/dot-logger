"use strict";
// Thin view of /api/status (dot_logger.viewer.dashboard.v1). Values are
// rendered as text only. A value the upstream did not send is shown as
// "not observed", never as zero.
(function () {
  const SCHEMA = "dot_logger.viewer.dashboard.v1";
  const POLL_MS = 1000;
  const CAPTURE = ["accepted", "accepted_bytes", "skipped", "rejected",
    "rejected_capacity", "rejected_contract", "rejected_lifecycle", "acquire_rejected",
    "acquire_rejected_capacity", "acquire_rejected_contract", "acquire_rejected_lifecycle"];
  const TOTALS = ["records", "assigned", "complete", "failed", "dropped", "unknown",
    "pending", "busy", "expired_records"];
  const BAD = new Set(["rejected", "rejected_capacity", "acquire_rejected",
    "acquire_rejected_capacity", "failed", "dropped"]);
  let viewerLostAt = null;

  function el(tag, cls, text) {
    const node = document.createElement(tag);
    if (cls) node.className = cls;
    if (text !== undefined) node.textContent = text;
    return node;
  }
  function fill(id, ...children) {
    const node = document.getElementById(id);
    node.replaceChildren(...children);
    return node;
  }
  function badge(state, reason) {
    const name = typeof state === "string" ? state : "Unavailable";
    return el("span", "badge " + name.toLowerCase(), reason ? name + ": " + reason : name);
  }
  function time(ms) {
    return new Date(ms).toLocaleTimeString();
  }
  function seconds(ms) {
    return Math.max(0, Math.round(ms / 1000)) + " s";
  }
  // Session-monotonic nanoseconds (a decimal string) as seconds.
  function ns(value) {
    if (typeof value !== "string" || !/^[0-9]+$/.test(value)) return "not observed";
    const n = BigInt(value);
    const frac = ((n % 1000000000n) / 1000000n).toString().padStart(3, "0");
    return (n / 1000000000n).toString() + "." + frac + " s";
  }
  function pairs(dl, entries) {
    for (const [name, value] of entries) {
      dl.append(el("dt", null, name), el("dd", null, value));
    }
    return dl;
  }
  function counts(keys, values) {
    const table = el("table");
    for (const key of keys) {
      const value = values ? values[key] : undefined;
      const present = typeof value === "string";
      // Highlight only a nonzero loss or Unknown count.
      const alert = present && value !== "0";
      const row = el("tr", alert && key === "unknown" ? "unknown" : alert && BAD.has(key) ? "bad" : null);
      row.append(el("th", null, key),
        el("td", present ? "num" : "absent", present ? value : "— not observed"));
      table.append(row);
    }
    return table;
  }
  // One Observed/Stale/Unavailable observation with its counts.
  function observation(title, value, keys) {
    const box = el("div", "card");
    const head = el("h3", null, title + " ");
    head.append(badge(value && value.state, value && value.reason));
    if (value && value.overflowed === true) head.append(badge("Saturated"));
    box.append(head);
    if (value && value.state !== "Unavailable") box.append(counts(keys, value));
    return box;
  }

  function render(p) {
    const poll = p.poll || {};
    const last = p.last;
    const status = [];
    if (viewerLostAt !== null) {
      // The viewer stopped answering: the upstream state is unknown.
      status.push(badge("Failed", "viewer unreachable"),
        el("span", "error", " since " + time(viewerLostAt)));
    } else if (poll.state === "ok") {
      status.push(badge("Observed", "connected"));
    } else if (poll.state === "pending") {
      status.push(badge("Pending", "waiting for the first status"));
    } else if (poll.failure) {
      status.push(badge("Failed", poll.failure.reason),
        el("span", "error", " at " + time(poll.failure.at_unix_ms)));
    }
    if (last) {
      const now = viewerLostAt !== null ? Date.now() : p.now_unix_ms;
      status.push(el("span", "note", " · last status " + time(last.received_at_unix_ms) +
        " (" + seconds(now - last.received_at_unix_ms) + " ago)"));
    }
    fill("connection", ...status);
    const stale = viewerLostAt !== null || poll.state === "failed";
    document.getElementById("main").classList.toggle("stale", stale);
    // Values below keep the states the upstream reported at that time.
    const notice = document.getElementById("notice");
    notice.hidden = !(stale && last);
    notice.textContent = stale && last
      ? "Not current: the values below are from the last status at " +
        time(last.received_at_unix_ms) + ". States shown are as reported then."
      : "";

    const identity = pairs(el("dl"), [["endpoint", String(p.upstream)]]);
    if (last) {
      pairs(identity, [["session", last.session], ["revision", last.revision],
        ["posted revision", typeof last.metrics.posted_revision === "string"
          ? last.metrics.posted_revision : "not tracked"]]);
    }
    fill("identity", ...identity.childNodes);
    if (!last) {
      fill("latest", el("p", "note", "No status received yet."));
      fill("windows");
      fill("snapshots");
      return;
    }
    const m = last.metrics;
    if (m.schema !== "dot_logger.status.v1") {
      fill("latest", badge(m.state, m.reason));
      fill("windows", el("p", "note", "No runtime metrics from this host."));
      fill("snapshots");
      return;
    }
    const latest = m.latest || {};
    const head = el("p");
    head.append(badge(latest.state, latest.reason));
    if (latest.state !== "Unavailable") {
      head.append(el("span", null, "at " + ns(latest.at_ns) + ", revision " +
        latest.revision + ", epoch " + latest.epoch));
      const grid = el("div", "cards");
      grid.append(observation("Capture", latest.capture, CAPTURE),
        observation("All outputs", latest.outputs, TOTALS),
        observation("Required outputs", latest.required_outputs, TOTALS));
      fill("latest", head, grid);
    } else {
      fill("latest", head);
    }
    fill("windows", ...(m.windows || []).map(function (w) {
      const card = el("div", "card");
      const title = el("h3", null, w.seconds + " s window ");
      title.append(badge(w.state, w.reason));
      card.append(title);
      if (w.state !== "Unavailable") {
        card.append(el("p", "note", ns(w.from_ns) + " → " + ns(w.to_ns)));
        if (w.counts && w.counts.overflowed === true) card.append(badge("Saturated"));
        card.append(counts(CAPTURE, w.counts));
      }
      return card;
    }));
    const snapshots = m.snapshots || [];
    if (snapshots.length === 0) {
      fill("snapshots", el("p", "note", "No snapshots held."));
    } else {
      const table = el("table");
      const header = el("tr");
      for (const name of ["ID", "revision", "epoch", "from", "to"]) header.append(el("th", null, name));
      table.append(header);
      for (const s of snapshots) {
        const row = el("tr");
        for (const value of [s.id, s.revision, s.epoch, ns(s.from_ns), ns(s.to_ns)]) {
          row.append(el("td", null, value));
        }
        table.append(row);
      }
      fill("snapshots", table);
    }
  }

  let latest = null;
  async function refresh() {
    try {
      const reply = await fetch("api/status", { cache: "no-store" });
      const body = await reply.json();
      if (!reply.ok || body.schema !== SCHEMA) throw new Error("bad reply");
      viewerLostAt = null;
      latest = body;
    } catch (error) {
      if (viewerLostAt === null) viewerLostAt = Date.now();
    }
    if (latest) render(latest);
    else if (viewerLostAt !== null) fill("connection", badge("Failed", "viewer unreachable"));
    setTimeout(refresh, POLL_MS);
  }
  refresh();
})();
