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

// Screenshot lightbox
(function () {
  var dlg = document.getElementById("lightbox");
  if (!dlg || !dlg.showModal) return;
  var img = document.getElementById("lightbox-img");
  var cap = document.getElementById("lightbox-cap");
  document.addEventListener("click", function (ev) {
    var shot = ev.target.closest && ev.target.closest(".shot");
    if (shot) {
      var lang = document.documentElement.getAttribute("lang") === "vi" ? "vi" : "en";
      img.src = shot.getAttribute("data-full");
      img.alt = cap.textContent = shot.getAttribute("data-" + lang) || "";
      dlg.showModal();
      return;
    }
    // Click on the backdrop or the close button closes the dialog.
    if (ev.target === dlg || (ev.target.id === "lightbox-close")) dlg.close();
  });
})();
