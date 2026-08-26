/*
 * Makes query placeholders editable, and shareable through the URL.
 *
 * The list-queries shortcode emits each PromQL placeholder as
 *
 *   <span contenteditable="true" class="replaceable"
 *         data-replace="mzSqlPrefix">mz_</span>
 *
 * This script gives those spans two things the attribute alone does not:
 *
 *   - An edit propagates to every occurrence of the same placeholder on the
 *     page. mzSqlPrefix appears 25 times on Common Queries; editing it once
 *     is the point of the exercise.
 *   - The URL query string tracks the edits, so a reader can pin values up
 *     front (?mzSqlPrefix=v2_mz_&mzNamespaceList=materialize-prod) and can
 *     equally well edit in place and then copy the URL out of the address
 *     bar. Only placeholders that differ from the page default appear, so
 *     the link stays readable and unrelated parameters are left alone.
 *
 * Values are only ever read and written as text, so nothing from the URL is
 * parsed as HTML, and a paste cannot smuggle markup into the page. Copying a
 * code block picks the current values up for free: the theme's clipboard
 * handler copies `code.textContent`.
 */
(function () {
  "use strict";

  // Placeholder names are identifiers. Anything else is not one of ours, and
  // refusing it also keeps the attribute selector below injection-free.
  var NAME_PATTERN = /^[A-Za-z_][A-Za-z0-9_]*$/;

  // These land inside a single-line code span; don't let a URL blow it out.
  var MAX_VALUE_LENGTH = 256;

  // Placeholder name -> the value the page was built with. Captured before
  // any override is applied, so the URL can carry only real differences.
  var defaults = {};

  // The value a span held when it took focus, for Escape to restore.
  var valueAtFocus = null;

  function spansFor(name) {
    return document.querySelectorAll(
      'span.replaceable[data-replace="' + name + '"]'
    );
  }

  function normalize(value) {
    return value.replace(/\s+/g, " ").trim().slice(0, MAX_VALUE_LENGTH);
  }

  function mark(span, value) {
    var name = span.dataset.replace;
    if (value === defaults[name]) {
      delete span.dataset.replaced;
      span.title = name;
    } else {
      span.dataset.replaced = "custom";
      span.title = name + " (customized)";
    }
  }

  /*
   * Writes `value` to every span for `name`. The span being typed into is
   * marked but not rewritten: assigning textContent would drop the caret.
   */
  function setValue(name, value, editing) {
    spansFor(name).forEach(function (span) {
      if (span !== editing) {
        span.textContent = value;
      }
      mark(span, value);
    });
  }

  function syncURL() {
    var params = new URLSearchParams(window.location.search);

    Object.keys(defaults).forEach(function (name) {
      var span = spansFor(name)[0];
      if (!span) {
        return;
      }
      if (span.textContent === defaults[name]) {
        params.delete(name);
      } else {
        params.set(name, span.textContent);
      }
    });

    var query = params.toString();
    window.history.replaceState(
      null,
      "",
      window.location.pathname +
        (query ? "?" + query : "") +
        window.location.hash
    );
  }

  function onInput(event) {
    var span = event.currentTarget;
    // contenteditable can leave nodes behind that we never asked for; reading
    // textContent flattens them for the spans we propagate to, and the blur
    // handler flattens the edited span itself.
    setValue(span.dataset.replace, span.textContent, span);
    syncURL();
  }

  function onKeyDown(event) {
    if (event.key === "Enter") {
      // A newline inside a one-line code span is never what was meant.
      event.preventDefault();
      event.currentTarget.blur();
    } else if (event.key === "Escape") {
      event.preventDefault();
      setValue(event.currentTarget.dataset.replace, valueAtFocus);
      syncURL();
      event.currentTarget.blur();
    }
  }

  function onPaste(event) {
    // Insert the plain text; the clipboard's HTML flavor is not welcome here.
    event.preventDefault();
    var text = (event.clipboardData || window.clipboardData).getData("text");
    document.execCommand("insertText", false, normalize(text));
  }

  function onFocus(event) {
    valueAtFocus = event.currentTarget.textContent;
  }

  /*
   * Book's clipboard helper binds `pre.focus` to a click on the <pre>, so that
   * a keyboard copy has something to copy from. The <pre> carries a tabindex
   * and so really does take focus — which blurs the span the click just landed
   * in, before a single character can be typed. Keep the click from reaching
   * the <pre>; clicking anywhere else in the block still focuses it.
   */
  function onClick(event) {
    event.stopPropagation();
  }

  /*
   * Book's search binds a document-level keypress handler that jumps focus to
   * the search box when one of its hotkeys ("s" and "/") is typed. Its "am I
   * already in a field?" guard tests `event.target.value !== undefined`, which
   * a contenteditable span fails — so typing `mz_storage` or a regex with a
   * slash would lose focus mid-edit. The handler is on document, so stopping
   * the event here keeps it from ever running, with no fork of the theme.
   */
  function onKeyPress(event) {
    event.stopPropagation();
  }

  function onBlur(event) {
    var span = event.currentTarget;
    var value = normalize(span.textContent);
    if (value !== span.textContent) {
      span.textContent = value;
    }
    setValue(span.dataset.replace, value, null);
    syncURL();
  }

  function applyOverrides() {
    new URLSearchParams(window.location.search).forEach(function (value, name) {
      if (NAME_PATTERN.test(name) && name in defaults) {
        setValue(name, normalize(value), null);
      }
    });
  }

  function init() {
    document.querySelectorAll("span.replaceable").forEach(function (span) {
      var name = span.dataset.replace;
      if (!(name in defaults)) {
        defaults[name] = span.textContent;
      }

      if (span.isContentEditable) {
        span.spellcheck = false;
        span.setAttribute("role", "textbox");
        span.setAttribute("aria-label", name);
        span.addEventListener("input", onInput);
        span.addEventListener("keydown", onKeyDown);
        span.addEventListener("keypress", onKeyPress);
        span.addEventListener("click", onClick);
        span.addEventListener("paste", onPaste);
        span.addEventListener("focus", onFocus);
        span.addEventListener("blur", onBlur);
      }
    });

    applyOverrides();
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", init);
  } else {
    init();
  }
})();
