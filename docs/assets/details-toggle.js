/*
 * Adds "Expand all" and "Collapse all" buttons to a page with many <details>
 * blocks.
 *
 * List of Metrics folds each metric's labels and example queries into one,
 * hundreds to the page, and opening them one at a time is the only other way
 * to read them all. The buttons go under the page title rather than next to
 * the first block, so a reader finds them in the same place on every page.
 *
 * Both buttons reach nested blocks too, so "Expand all" leaves nothing folded.
 * The body partial decides which pages get this script; see its threshold.
 */
(function () {
  "use strict";

  var article = document.querySelector(".book-article");
  var blocks = article ? article.querySelectorAll("details") : [];
  if (!blocks.length) {
    return;
  }

  var toolbar = document.createElement("div");
  toolbar.className = "details-toggle";
  toolbar.setAttribute("role", "group");
  toolbar.setAttribute("aria-label", "Collapsible sections");
  toolbar.append(button("Expand all", true), button("Collapse all", false));

  var title = article.querySelector(":scope > h1");
  if (title) {
    title.after(toolbar);
  } else {
    article.prepend(toolbar);
  }

  function button(label, open) {
    var element = document.createElement("button");
    element.type = "button";
    element.textContent = label;
    element.addEventListener("click", function () {
      blocks.forEach(function (block) {
        block.open = open;
      });
    });
    return element;
  }
})();
