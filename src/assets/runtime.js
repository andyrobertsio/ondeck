// deck runtime: navigation, fragment stepping, slide transitions, scale-to-fit.
(function () {
  "use strict";

  var slides = Array.prototype.slice.call(document.querySelectorAll(".slide"));
  if (!slides.length) return;
  var stage = document.querySelector(".stage");

  // Scale one slide's slot content down uniformly until it fits its grid cell.
  function fitSlide(slide) {
    slide.querySelectorAll(".fit").forEach(function (el) {
      el.style.transform = "";
      var slot = el.parentElement;
      var w = el.scrollWidth, h = el.scrollHeight;
      if (!w || !h) return; // display:none slides measure 0 — skip
      var k = Math.min(1, slot.clientWidth / w, slot.clientHeight / h);
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

  // --- Fragments ---------------------------------------------------------
  function stepsOf(slide) {
    var set = {};
    slide.querySelectorAll(".fragment").forEach(function (f) {
      set[+f.dataset.step || 0] = true;
    });
    return Object.keys(set).map(Number).sort(function (a, b) { return a - b; });
  }
  function applyFrags(slide, shown) {
    var steps = stepsOf(slide);
    var max = shown > 0 ? steps[shown - 1] : -Infinity;
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
    // fade is a dissolve: the incoming stays opaque, the outgoing fades out on
    // top of it, so the slide background never gaps to the stage/frame.
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
    // Finalize any in-flight transition: drop transient classes everywhere.
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
    fitSlide(slides[current]); // fit now that it's displayed
    if (num) num.textContent = current + 1 + " / " + slides.length;
    if (bar) bar.style.width = ((current + 1) / slides.length) * 100 + "%";
    if (window.location.hash !== "#" + (current + 1)) {
      history.replaceState(null, "", "#" + (current + 1));
    }
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

    setActive(target, revealAll); // current = target; fragments + fit on incoming

    // Next frame: enable transitions and move to the resting/exit states.
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
    setTimeout(done, 900); // fallback if transitionend doesn't fire
  }

  function next() {
    var total = stepsOf(slides[current]).length;
    if (shown < total) { shown++; applyFrags(slides[current], shown); }
    else go(current + 1, false);
  }
  function prev() {
    if (shown > 0) { shown--; applyFrags(slides[current], shown); }
    else go(current - 1, true); // entering a previous slide shows it complete
  }

  document.addEventListener("keydown", function (e) {
    switch (e.key) {
      case "ArrowRight": case "PageDown": case " ":
        e.preventDefault(); next(); break;
      case "ArrowLeft": case "PageUp":
        e.preventDefault(); prev(); break;
      case "Home":
        e.preventDefault(); go(0, false); break;
      case "End":
        e.preventDefault(); go(slides.length - 1, true); break;
    }
  });

  document.addEventListener("click", function (e) {
    if (e.target.closest("a")) return;
    if (e.clientX < window.innerWidth / 3) prev();
    else next();
  });

  // Initial slide (1-based hash), no fragments revealed, no transition.
  var start = parseInt((window.location.hash || "").slice(1), 10);
  start = isNaN(start) ? 0 : Math.max(0, Math.min(slides.length - 1, start - 1));
  slides.forEach(function (s, idx) { s.classList.toggle("active", idx === start); });
  setActive(start, false);
})();
