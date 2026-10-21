/*
 * Gives every highlighted code block a "Copy" button.
 *
 * The book theme already copies a whole block when the copy shortcut is
 * pressed with nothing selected (see its clipboard.js), but nothing on the
 * page says so. The button copies the block in one click, and reads "Copied"
 * for a moment after.
 *
 * It copies the <code> element's text, so the button's own label stays out of
 * it, and a replaceable placeholder comes along with whatever value a reader
 * typed into it. The block's trailing newline is dropped, so a command pasted
 * into a terminal waits for Enter instead of running.
 *
 * Without clipboard access, as on a page served over plain HTTP or in a
 * browser that refuses the write, the button selects the block instead, ready
 * for the copy shortcut.
 */
(function () {
  "use strict";

  var LABEL = "Copy";
  var RESET_AFTER_MS = 2000;

  document.querySelectorAll(".highlight").forEach(function (block) {
    var code = block.querySelector("pre > code");
    if (!code) {
      return;
    }

    var button = document.createElement("button");
    button.type = "button";
    button.className = "code-copy";
    button.textContent = LABEL;

    var reset = null;

    function show(label) {
      button.textContent = label;
      clearTimeout(reset);
      reset = setTimeout(function () {
        button.textContent = LABEL;
      }, RESET_AFTER_MS);
    }

    function select() {
      window.getSelection().selectAllChildren(code);
      show("Selected");
    }

    button.addEventListener("click", function () {
      if (!navigator.clipboard) {
        select();
        return;
      }
      var text = code.textContent.replace(/\n$/, "");
      navigator.clipboard.writeText(text).then(function () {
        show("Copied");
      }, select);
    });
    block.append(button);
  });
})();
