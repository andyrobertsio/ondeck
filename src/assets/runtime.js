// deck runtime: navigation, fragment stepping, slide transitions, scale-to-fit,
// and a two-window presenter view (audience + notes/dashboard, kept in sync).
(function () {
  "use strict";

  var slides = Array.prototype.slice.call(document.querySelectorAll(".slide"));
  if (!slides.length) return;
  var stage = document.querySelector(".stage");

  // Scale one slide's block content down uniformly until it fits its grid cell.
  function fitSlide(slide) {
    slide.querySelectorAll(".fit").forEach(function (el) {
      el.style.transform = "";
      var block = el.parentElement;
      var w = el.scrollWidth, h = el.scrollHeight;
      if (!w || !h) return; // display:none slides measure 0 — skip
      var k = Math.min(1, block.clientWidth / w, block.clientHeight / h);
      if (k < 1) el.style.transform = "scale(" + k + ")";
    });
  }
  function fitAll() { slides.forEach(fitSlide); }

  window.addEventListener("resize", fitAll);
  if (document.readyState === "complete") fitAll();
  else window.addEventListener("load", fitAll);

  var params = new URLSearchParams(window.location.search);
  if (params.get("mode") === "print") {
    document.body.classList.add("print");
    fitAll();
    return;
  }

  // ?shot=N — render only slide N (1-based) at full size for PNG capture.
  var shot = params.get("shot");
  if (shot !== null) {
    document.body.classList.add("print", "shot");
    var target = slides[(parseInt(shot, 10) || 1) - 1];
    if (target) {
      target.classList.add("shot-active");
      fitSlide(target);
    }
    return;
  }

  // ?present=1 — this window is the presenter dashboard (notes + previews).
  var isPresenter = params.get("present") === "1";

  // --- Fragments ---------------------------------------------------------
  function stepsOf(slide) {
    var set = {};
    slide.querySelectorAll(".fragment").forEach(function (f) {
      set[+f.dataset.step || 0] = true;
    });
    return Object.keys(set).map(Number).sort(function (a, b) { return a - b; });
  }
  function applyFrags(slide, shownN) {
    var steps = stepsOf(slide);
    var max = shownN > 0 ? steps[shownN - 1] : -Infinity;
    slide.querySelectorAll(".fragment").forEach(function (f) {
      f.classList.toggle("revealed", (+f.dataset.step || 0) <= max);
    });
  }

  // --- Slide transitions -------------------------------------------------
  var TX_CLASSES = ["leaving", "notrans", "from-fade", "from-right", "from-left", "to-fade", "to-left", "to-right"];
  function effectFor(slide) {
    return slide.dataset.transition || (stage && stage.dataset.transition) || "none";
  }
  function fromClass(effect, fwd) {
    if (effect === "fade") return "";
    if (effect === "slide") return fwd ? "from-right" : "from-left";
    return "";
  }
  function toClass(effect, fwd) {
    if (effect === "fade") return "to-fade";
    if (effect === "slide") return fwd ? "to-left" : "to-right";
    return "";
  }
  function snapClean() {
    slides.forEach(function (s) { s.classList.remove.apply(s.classList, TX_CLASSES); });
  }

  var current = 0;
  var shown = 0; // fragment steps revealed on the current slide

  var num = document.querySelector(".deck-number");
  var bar = document.querySelector(".deck-progress > i");

  function setActive(i, revealAll) {
    current = i;
    var total = stepsOf(slides[current]).length;
    shown = revealAll ? total : 0;
    applyFrags(slides[current], shown);
    fitSlide(slides[current]);
    if (num) num.textContent = current + 1 + " / " + slides.length;
    if (bar) bar.style.width = ((current + 1) / slides.length) * 100 + "%";
    var hash = "#" + (current + 1);
    if (window.location.hash !== hash) {
      history.replaceState(null, "", location.pathname + location.search + hash);
    }
    changed();
  }

  function go(target, revealAll) {
    target = Math.max(0, Math.min(slides.length - 1, target));
    if (target === current) return;

    var fwd = target > current;
    var incoming = slides[target];
    var outgoing = slides[current];
    var effect = effectFor(incoming);
    snapClean();

    if (effect === "none") {
      slides.forEach(function (s, idx) { s.classList.toggle("active", idx === target); });
      setActive(target, revealAll);
      return;
    }

    var fromC = fromClass(effect, fwd);
    var toC = toClass(effect, fwd);

    incoming.classList.add("active", "notrans");
    if (fromC) incoming.classList.add(fromC);
    outgoing.classList.remove("active");
    outgoing.classList.add("leaving", "notrans");

    setActive(target, revealAll);

    requestAnimationFrame(function () {
      requestAnimationFrame(function () {
        incoming.classList.remove("notrans");
        outgoing.classList.remove("notrans");
        if (fromC) incoming.classList.remove(fromC);
        if (toC) outgoing.classList.add(toC);
      });
    });

    var done = function () {
      outgoing.removeEventListener("transitionend", done);
      outgoing.classList.remove("leaving", "to-fade", "to-left", "to-right");
    };
    outgoing.addEventListener("transitionend", done);
    setTimeout(done, 900);
  }

  function next() {
    var total = stepsOf(slides[current]).length;
    if (shown < total) { shown++; applyFrags(slides[current], shown); changed(); }
    else go(current + 1, false);
  }
  function prev() {
    if (shown > 0) { shown--; applyFrags(slides[current], shown); changed(); }
    else go(current - 1, true);
  }

  // --- Cross-window sync (audience <-> presenter) ------------------------
  var SELF = String(Math.random()).slice(2);
  var peer = (window.opener && window.opener !== window) ? window.opener : null;
  var bc = null;
  try {
    if (location.protocol !== "file:" && "BroadcastChannel" in window) {
      bc = new BroadcastChannel("ondeck-present");
    }
  } catch (e) { /* ignore */ }
  var applyingRemote = false;

  function postState() {
    var msg = { ondeck: true, src: SELF, slide: current, step: shown };
    if (peer) { try { peer.postMessage(msg, "*"); } catch (e) { peer = null; } }
    if (bc) { try { bc.postMessage(msg); } catch (e) { /* ignore */ } }
  }
  function onRemote(msg) {
    if (!msg || !msg.ondeck || msg.src === SELF) return;
    applyingRemote = true;
    if (msg.slide !== current) go(msg.slide, false);
    var total = stepsOf(slides[current]).length;
    shown = Math.max(0, Math.min(total, msg.step));
    applyFrags(slides[current], shown);
    refreshPresenter();
    applyingRemote = false;
  }
  window.addEventListener("message", function (e) { onRemote(e.data); });
  if (bc) bc.onmessage = function (e) { onRemote(e.data); };

  // Called whenever (current, shown) changes — refresh the dashboard and tell
  // the peer, unless we're applying a change the peer just sent us.
  function changed() {
    refreshPresenter();
    if (!applyingRemote) postState();
  }

  // --- Presenter dashboard ----------------------------------------------
  var pv = null; // refs to dashboard nodes, built lazily
  function buildPresenter() {
    document.body.classList.add("presenter");
    var root = document.createElement("div");
    root.className = "presenter-view";
    root.innerHTML =
      '<div class="pv-main">' +
      '<div class="pv-col pv-current"><div class="pv-label">Current <span class="pv-count"></span></div><div class="pv-screen" id="pvNow"></div></div>' +
      '<div class="pv-col pv-aside">' +
      '<div class="pv-next-wrap"><div class="pv-label">Next</div><div class="pv-screen pv-small" id="pvNext"></div></div>' +
      '<div class="pv-meta"><div class="pv-timer" id="pvTimer">00:00</div><div class="pv-clock" id="pvClock"></div></div>' +
      "</div></div>" +
      '<div class="pv-notes" id="pvNotes"></div>';
    document.body.appendChild(root);
    pv = {
      now: root.querySelector("#pvNow"),
      next: root.querySelector("#pvNext"),
      notes: root.querySelector("#pvNotes"),
      count: root.querySelector(".pv-count"),
      timer: root.querySelector("#pvTimer"),
      clock: root.querySelector("#pvClock")
    };
    var start = Date.now();
    function tick() {
      var s = Math.max(0, Math.floor((Date.now() - start) / 1000));
      var mm = String(Math.floor(s / 60)).padStart(2, "0");
      var ss = String(s % 60).padStart(2, "0");
      pv.timer.textContent = mm + ":" + ss;
      pv.clock.textContent = new Date().toLocaleTimeString();
    }
    tick();
    setInterval(tick, 250);
  }

  function renderPreview(host, idx, frags) {
    host.textContent = "";
    if (idx < 0 || idx >= slides.length) {
      host.classList.add("pv-empty-screen");
      return;
    }
    host.classList.remove("pv-empty-screen");
    var mini = document.createElement("div");
    mini.className = "stage";
    var clone = slides[idx].cloneNode(true);
    clone.classList.add("active");
    TX_CLASSES.forEach(function (c) { clone.classList.remove(c); });
    var n = clone.querySelector(".notes"); if (n) n.remove();
    mini.appendChild(clone);
    host.appendChild(mini);
    applyFrags(clone, frags || 0);
    requestAnimationFrame(function () { fitSlide(clone); });
  }

  function refreshPresenter() {
    if (!isPresenter || !pv) return;
    renderPreview(pv.now, current, shown);
    renderPreview(pv.next, current + 1, 0);
    var note = slides[current].querySelector(".notes");
    pv.notes.innerHTML = note ? note.innerHTML : '<p class="pv-empty">No notes for this slide.</p>';
    pv.count.textContent = current + 1 + " / " + slides.length;
  }

  // --- Presenter window + fullscreen ------------------------------------
  function openPresenter() {
    if (isPresenter) return;
    var base = location.href.split("#")[0].split("?")[0];
    var w = window.open(base + "?present=1", "ondeck-presenter", "width=1100,height=760");
    if (w) { peer = w; setTimeout(postState, 500); }
  }
  function toggleFullscreen() {
    var el = document.documentElement;
    if (document.fullscreenElement) { document.exitFullscreen(); }
    else if (el.requestFullscreen) { el.requestFullscreen(); }
  }

  document.addEventListener("keydown", function (e) {
    if (e.metaKey || e.ctrlKey || e.altKey) return;
    switch (e.key) {
      case "ArrowRight": case "PageDown": case " ":
        e.preventDefault(); next(); break;
      case "ArrowLeft": case "PageUp":
        e.preventDefault(); prev(); break;
      case "Home":
        e.preventDefault(); go(0, false); break;
      case "End":
        e.preventDefault(); go(slides.length - 1, true); break;
      case "p": case "P":
        e.preventDefault(); openPresenter(); break;
      case "f": case "F":
        e.preventDefault(); toggleFullscreen(); break;
    }
  });

  // Click-to-advance: audience only (the dashboard has its own controls/keys).
  if (!isPresenter) {
    document.addEventListener("click", function (e) {
      if (e.target.closest("a")) return;
      if (e.clientX < window.innerWidth / 3) prev();
      else next();
    });
  }

  // --- Init --------------------------------------------------------------
  if (isPresenter) buildPresenter();
  var start = parseInt((window.location.hash || "").slice(1), 10);
  start = isNaN(start) ? 0 : Math.max(0, Math.min(slides.length - 1, start - 1));
  slides.forEach(function (s, idx) { s.classList.toggle("active", idx === start); });
  setActive(start, false);
})();
