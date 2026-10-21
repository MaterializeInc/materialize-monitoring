/*
 * Gives every highlighted code block a "Select all" button.
 *
 * The book theme already copies a whole block when the copy shortcut is
 * pressed with nothing selected (see its clipboard.js), but nothing on the
 * page says so, and a drag-select across a long query is easy to fumble. The
 * button selects the block's text outright, so the copy shortcut or the
 * context menu picks up exactly the block.
 *
 * The selection covers the <code> element's children, not the <pre>, so the
 * button stays out of it. Replaceable placeholders are text inside <code>, so
 * whatever value a reader typed into one is selected with the rest.
 */
(function () {
  "use strict";

  document.querySelectorAll(".highlight").forEach(function (block) {
    var code = block.querySelector("pre > code");
    if (!code) {
      return;
    }

    var button = document.createElement("button");
    button.type = "button";
    button.className = "code-select-all";
    button.textContent = "Select all";
    button.addEventListener("click", function () {
      window.getSelection().selectAllChildren(code);
    });
    block.append(button);
  });
})();
