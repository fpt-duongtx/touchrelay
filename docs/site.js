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
