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
  let snapshotKey = null;
  const CHANGES = ["accepted_rate", "accepted_bytes_rate", "completed_rate",
    "intentional_skip_rate", "capacity_rejection_rate", "cpu_rate", "latency_ns", "memory_bytes"];
  const SIDES = [["id", "ID"], ["session", "session"], ["revision", "revision"],
    ["epoch", "epoch"], ["window", "window"], ["application_input_id", "application input"],
    ["environment_id", "environment"], ["workload_id", "workload"], ["target_id", "target"],
    ["boundary", "boundary"]];

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
      compareForm(null);
      return;
    }
    const m = last.metrics;
    if (m.schema !== "dot_logger.status.v1") {
      fill("latest", badge(m.state, m.reason));
      fill("windows", el("p", "note", "No runtime metrics from this host."));
      fill("snapshots");
      compareForm(null);
      return;
    }
    compareForm(m.snapshots || []);
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

  // ---- Read-only lookups: Compare and PlanCheck preview ----
  function snapshotLabel(s) {
    return "#" + s.id + " · revision " + s.revision + " · " + ns(s.from_ns) + " → " + ns(s.to_ns);
  }
  function choose(snapshots, selected) {
    const select = el("select");
    for (const s of snapshots) {
      const option = el("option", null, snapshotLabel(s));
      option.value = s.id;
      select.append(option);
    }
    select.value = snapshots[selected].id;
    return select;
  }
  function labelled(text, control) {
    const label = el("label", null, text + " ");
    label.append(control);
    return label;
  }
  // Rebuilt only when the held snapshots change, so a choice survives refreshes.
  function compareForm(snapshots) {
    const key = snapshots === null ? "none" : snapshots.map(function (s) { return s.id; }).join(",");
    if (key === snapshotKey) return;
    snapshotKey = key;
    if (snapshots === null) {
      fill("compare-form", el("p", "note", "No snapshot list from this host."));
      return;
    }
    if (snapshots.length < 2) {
      fill("compare-form", el("p", "note", snapshots.length === 0
        ? "This host holds no snapshots. A comparison needs two."
        : "This host holds one snapshot. A comparison needs two."));
      return;
    }
    const before = choose(snapshots, snapshots.length - 2);
    const after = choose(snapshots, snapshots.length - 1);
    const go = el("button", null, "Compare");
    go.type = "button";
    go.addEventListener("click", function () {
      lookup("compare", "api/compare?before=" + encodeURIComponent(before.value) +
        "&after=" + encodeURIComponent(after.value), renderComparison);
    });
    fill("compare-form", labelled("Before", before), labelled("After", after), go);
  }
  const FAILURES = {
    compare: { gone: "A selected snapshot is no longer held by the host.",
      unsupported: "This host does not provide comparisons." },
    plan: { gone: "No plan with this ID is registered on the host.",
      unsupported: "This host does not provide check plans." },
  };
  function failure(kind, doc) {
    const text = (FAILURES[kind] || {})[doc.reason] ||
      (doc.reason === "viewer_busy" ? "The viewer is busy with other lookups. Try again." : null);
    const box = el("p");
    box.append(badge("Failed", doc.reason), el("span", "error", text ? " " + text : ""),
      el("span", "note", " at " + time(doc.at_unix_ms)));
    return box;
  }
  // Only the newest lookup of each kind may update its result (RV-169).
  const newest = { compare: 0, plan: 0 };
  async function lookup(kind, url, render) {
    const ticket = ++newest[kind];
    fill(kind, el("p", "note", "Asking the host…"));
    let shown;
    try {
      const reply = await fetch(url, { cache: "no-store" });
      const doc = await reply.json();
      if (!reply.ok || doc.schema !== "dot_logger.viewer.request.v1" || doc.request !== kind) {
        throw new Error("bad reply");
      }
      shown = doc.outcome !== "ok" ? [failure(kind, doc)] : render(doc.report, doc.at_unix_ms);
    } catch (error) {
      shown = [badge("Failed", "viewer unreachable")];
    }
    if (ticket === newest[kind]) fill(kind, ...shown);
  }
  function change(name, c) {
    const row = el("tr");
    row.append(el("th", null, name));
    const state = el("td");
    state.append(badge(c && c.state, c && c.state === "Unavailable" ? c.reason : null));
    row.append(state);
    if (!c || c.state === "Unavailable") {
      row.append(el("td", "absent", "—"), el("td", "absent", "—"), el("td"));
      return row;
    }
    const origin = c.profile_id ? "profile " + c.profile_id
      : c.assumptions ? "assumptions " + c.assumptions : c.basis ? "basis " + c.basis : "";
    const value = el("td", "num", difference(name, c));
    value.title = "exact: " + c.numerator + " / " + c.denominator;
    row.append(el("td", null, c.direction), value, el("td", "note", origin));
    return row;
  }
  // The exact magnitude of after minus before is numerator/denominator, with
  // time in nanoseconds. Rates are shown per second; others in their unit.
  function difference(name, c) {
    if (!/^[0-9]+$/.test(c.numerator) || !/^[1-9][0-9]*$/.test(c.denominator)) return "—";
    const sign = c.direction === "Increase" ? "+" : c.direction === "Decrease" ? "−" : "";
    const perSecond = name.endsWith("_rate") && name !== "cpu_rate";
    if (name === "cpu_rate") return sign + c.numerator + " / " + c.denominator;
    const scale = perSecond ? 1000000000n : 1n;
    const thousandths = BigInt(c.numerator) * scale * 1000n / BigInt(c.denominator);
    const text = (thousandths / 1000n).toString() + "." +
      (thousandths % 1000n).toString().padStart(3, "0");
    const unit = name === "accepted_bytes_rate" ? " B/s" : perSecond ? " /s"
      : name === "latency_ns" ? " ns" : name === "memory_bytes" ? " B" : "";
    return sign + text + unit;
  }
  function side(s, key) {
    if (!s) return undefined;
    if (key === "window") return ns(s.from_ns) + " → " + ns(s.to_ns);
    const value = key in s ? s[key] : (s.conditions || {})[key];
    return value === undefined || value === null ? undefined : String(value);
  }
  function renderComparison(report, at) {
    const head = el("p");
    head.append(report.comparable === true ? badge("Observed", "comparable")
      : badge("Unavailable", "not comparable"), el("span", "note", " at " + time(at)));
    const differs = Object.keys(report.differences || {})
      .filter(function (k) { return report.differences[k] === true; });
    const conditions = el("p", differs.length ? "error" : "note", differs.length
      ? "Conditions differ: " + differs.join(", ") + "."
      : "No condition differences.");
    const sides = el("table");
    const top = el("tr");
    top.append(el("th"), el("th", null, "before"), el("th", null, "after"));
    sides.append(top);
    for (const [key, name] of SIDES) {
      const a = side(report.before, key);
      const b = side(report.after, key);
      const row = el("tr");
      const cls = a !== b ? "differs" : null;
      row.append(el("th", null, name), el("td", cls, a === undefined ? "—" : a),
        el("td", cls, b === undefined ? "—" : b));
      sides.append(row);
    }
    const changes = el("table");
    const header = el("tr");
    for (const name of ["change", "state", "direction", "after − before", "source"]) {
      header.append(el("th", null, name));
    }
    changes.append(header);
    for (const name of CHANGES) changes.append(change(name, (report.changes || {})[name]));
    const grid = el("div", "stack");
    const left = el("div", "card");
    left.append(el("h3", null, "Snapshots"), sides);
    const right = el("div", "card");
    right.append(el("h3", null, "Changes"), changes);
    grid.append(left, right);
    return [head, conditions, grid];
  }
  function renderPlan(report, at) {
    const head = el("p");
    head.append(badge("Pending", "preview only, not run"), el("span", "note", " at " + time(at)));
    const list = el("dl");
    for (const key of Object.keys(report)) {
      if (key === "schema" || key === "kind") continue;
      const value = report[key];
      pairs(list, [[key, value === null ? "none"
        : typeof value === "object" ? JSON.stringify(value) : String(value)]]);
    }
    return [head, list, el("p", "note",
      "Running a check uses the CLI: dot-logger check run --plan " + report.plan +
      " (Manage permission).")];
  }
  document.getElementById("plan-go").addEventListener("click", function () {
    const id = document.getElementById("plan-id").value.trim();
    if (!/^[0-9]+$/.test(id)) {
      fill("plan", el("p", "error", "Enter a plan ID (digits only)."));
      return;
    }
    lookup("plan", "api/plan?id=" + id, renderPlan);
  });

  // `latest` is the last status that rendered completely. A status the page
  // cannot render keeps it on screen with a separate error, and the next
  // refresh is always scheduled (RV-167).
  let latest = null;
  async function refresh() {
    let body = null;
    try {
      const reply = await fetch("api/status", { cache: "no-store" });
      body = await reply.json();
      if (!reply.ok || body.schema !== SCHEMA) throw new Error("bad reply");
      viewerLostAt = null;
    } catch (error) {
      body = null;
      if (viewerLostAt === null) viewerLostAt = Date.now();
    }
    try {
      if (body) {
        try {
          render(body);
          latest = body;
        } catch (error) {
          if (latest) render(latest);
          const header = document.getElementById("connection");
          header.prepend(badge("Failed", "status not understood"),
            el("span", "error", " at " + time(Date.now()) + " "));
        }
      } else if (latest) {
        render(latest);
      } else if (viewerLostAt !== null) {
        fill("connection", badge("Failed", "viewer unreachable"));
      }
    } finally {
      setTimeout(refresh, POLL_MS);
    }
  }
  refresh();
})();
