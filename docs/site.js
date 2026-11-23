// Language: follows the browser (vi → Vietnamese, otherwise English); the choice is remembered.
(function () {
  var root = document.documentElement;
  var saved = null;
  try { saved = localStorage.getItem("lang"); } catch (e) {}
  var nav = (navigator.language || "en").toLowerCase();
  var lang = saved || (nav.indexOf("vi") === 0 ? "vi" : "en");

  function apply(l) {
    root.setAttribute("lang", l);
    var btns = document.querySelectorAll(".lang button");
    for (var i = 0; i < btns.length; i++) {
      btns[i].setAttribute("aria-pressed", btns[i].getAttribute("data-lang") === l ? "true" : "false");
    }
    try { localStorage.setItem("lang", l); } catch (e) {}
  }

  document.addEventListener("click", function (ev) {
    var b = ev.target.closest && ev.target.closest(".lang button");
    if (b) apply(b.getAttribute("data-lang"));
  });
  apply(lang);
})();

// Screenshot lightbox: waits until the full-size image is loaded before opening, so it is never blank.
(function () {
  var dlg = document.getElementById("lightbox");
  if (!dlg || !dlg.showModal) return;
  var img = document.getElementById("lightbox-img");
  var cap = document.getElementById("lightbox-cap");
  var fail = document.createElement("p");
  fail.className = "lb-fail";
  dlg.querySelector("figure").appendChild(fail);

  function open(shot) {
    var lang = document.documentElement.getAttribute("lang") === "vi" ? "vi" : "en";
    var url = shot.getAttribute("data-full");
    var text = shot.getAttribute("data-" + lang) || "";
    cap.textContent = text;
    img.alt = text;
    fail.hidden = true;
    img.hidden = false;
    var loader = new Image();
    var done = false;
    function show(ok) {
      if (done) return;
      done = true;
      if (ok) {
        img.src = url;
      } else {
        img.hidden = true;
        fail.hidden = false;
        fail.innerHTML = (lang === "vi" ? "Không tải được ảnh. " : "Could not load the image. ") +
          '<a href="' + url + '" target="_blank" rel="noopener">' + (lang === "vi" ? "Mở ảnh ở tab mới" : "Open it in a new tab") + "</a>";
      }
      if (!dlg.open) dlg.showModal();
    }
    loader.onload = function () { show(true); };
    loader.onerror = function () { show(false); };
    loader.src = url;
    // Slow connection: open anyway after a moment and let the browser finish loading.
    setTimeout(function () { if (!done) { img.src = url; if (!dlg.open) dlg.showModal(); } }, 1500);
  }

  document.addEventListener("click", function (ev) {
    var shot = ev.target.closest && ev.target.closest(".shot");
    if (shot) { open(shot); return; }
    // Click on the backdrop or the close button closes the dialog.
    if (ev.target === dlg || ev.target.id === "lightbox-close") dlg.close();
  });
})();
