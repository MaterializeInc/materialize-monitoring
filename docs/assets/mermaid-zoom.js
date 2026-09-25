/*
 * Opens any Mermaid diagram in a full-window view that zooms and pans.
 *
 * The book theme renders each ```mermaid block as <pre class="mermaid">, and
 * Mermaid swaps the source for an SVG on window load. This script gives each
 * diagram an expand button. The button opens a modal <dialog>, renders the
 * diagram into it afresh, and hands the result to svg-pan-zoom: scroll or
 * pinch to zoom, drag to pan, double-click to zoom in, or use the controls in
 * the corner. Escape, the close button, or a click outside closes it.
 *
 * The inline diagram is left alone, so scrolling down the page never gets
 * captured by a diagram on the way past.
 *
 * Rendering afresh, rather than cloning the inline SVG, keeps element IDs
 * unique. Mermaid scopes each diagram's styles and arrowheads by ID, and a
 * clone would have the dialog's copy borrowing them from the page's.
 */
(function () {
  "use strict";

  var diagrams = document.querySelectorAll("pre.mermaid");
  if (!diagrams.length || typeof svgPanZoom !== "function") {
    return;
  }

  // `closedby="any"` closes on a click outside, where the browser supports
  // it. Escape and the close button work everywhere.
  var dialog = document.createElement("dialog");
  dialog.className = "mermaid-zoom";
  dialog.setAttribute("closedby", "any");
  dialog.setAttribute("aria-label", "Diagram");
  dialog.innerHTML =
    '<button type="button" class="mermaid-zoom-close" aria-label="Close">&times;</button>' +
    '<div class="mermaid-zoom-stage"></div>';
  document.body.append(dialog);

  var stage = dialog.querySelector(".mermaid-zoom-stage");
  dialog.querySelector(".mermaid-zoom-close").addEventListener("click", function () {
    dialog.close();
  });

  var panZoom = null;
  var renders = 0;

  diagrams.forEach(function (pre) {
    // Mermaid has not run yet (it waits for window load), so this is still
    // the diagram source. Keep it: the SVG that replaces it can't be re-read.
    var source = pre.textContent;

    var button = document.createElement("button");
    button.type = "button";
    button.className = "mermaid-zoom-open";
    button.title = "Expand diagram";
    button.setAttribute("aria-label", "Expand diagram");
    button.textContent = "⤢";
    button.addEventListener("click", function () {
      open(source);
    });

    var wrapper = document.createElement("div");
    wrapper.className = "mermaid-zoom-wrapper";
    pre.before(wrapper);
    wrapper.append(pre, button);
  });

  function open(source) {
    renders += 1;
    mermaid.render("mermaid-zoom-" + renders, source).then(function (result) {
      stage.innerHTML = result.svg;
      dialog.showModal();

      // Mermaid caps the width at the diagram's natural size. Fill the stage
      // instead, and let svg-pan-zoom fit the drawing inside it.
      var svg = stage.querySelector("svg");
      svg.style.maxWidth = "none";
      svg.setAttribute("width", "100%");
      svg.setAttribute("height", "100%");

      panZoom = svgPanZoom(svg, {
        controlIconsEnabled: true,
        zoomScaleSensitivity: 0.3,
        minZoom: 0.5,
        maxZoom: 20,
      });
    }, function (error) {
      console.error("mermaid-zoom: could not render diagram", error);
    });
  }

  dialog.addEventListener("close", function () {
    if (panZoom) {
      panZoom.destroy();
      panZoom = null;
    }
    stage.replaceChildren();
  });

  window.addEventListener("resize", function () {
    if (panZoom) {
      panZoom.resize();
      panZoom.fit();
      panZoom.center();
    }
  });
})();
