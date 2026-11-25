//! Shared JavaScript building blocks for browser automation.
//!
//! The interactive-element detection logic (the `isVisible` predicate, the
//! `interactiveSelectors` array, and the `matched` collection) is identical in
//! every script that needs to find a clickable or typeable element. Keeping
//! those fragments in one place means a single edit updates all consumers and
//! removes the risk of the selectors drifting out of sync.
//!
//! The small fragments are plain `&'static str` constants so they can be
//! interpolated directly into a `format!()` call at compile time. Where the
//! caller needs a runtime value (an element index or a text payload) a builder
//! function is provided instead. The composite `EXTRACT_DOM_JS` script is a
//! `LazyLock<String>` because `concat!()` only accepts string literals, not
//! `const` items, so it cannot be assembled purely at compile time.

use std::sync::LazyLock;

/// The `isVisible` predicate shared by every script that filters DOM elements.
///
/// Returns `true` when the element is rendered, opaque, and at least partially
/// within the current viewport.
pub const VISIBLE_FN: &str = r"
const isVisible = (elem) => {
    if (!elem) return false;
    const style = window.getComputedStyle(elem);
    if (style.display === 'none' || style.visibility === 'hidden' || style.opacity === '0') return false;
    const rect = elem.getBoundingClientRect();
    return rect.width > 0 && rect.height > 0 && rect.bottom >= 0 && rect.top <= window.innerHeight;
};
";

/// The CSS selector list used to discover interactive elements.
pub const SELECTORS: &str = r#"
const interactiveSelectors = [
    'a[href]', 'button', 'input', 'textarea', 'select',
    '[role="button"]', '[role="link"]', '[role="checkbox"]', '[role="menuitem"]',
    '[role="tab"]', '[role="combobox"]', '[role="switch"]', '[tabindex]:not([tabindex="-1"])'
];
"#;

/// Collects every matched, visible interactive element into `matched`.
pub const MATCHED: &str = r"
const matched = Array.from(document.querySelectorAll(interactiveSelectors.join(',')))
    .filter(isVisible);
";

// Builds the full DOM-extraction script used by [`Browser::refresh_state`].
//
// This is the function form of [`EXTRACT_DOM_JS`]; it exists so callers that
// need a runtime value (e.g. tests) can compose the script without touching
// the constant.
// #[must_use]
// pub fn build_extract_dom_js() -> String {
//     format!(
//         "(() => {{\n{VISIBLE_FN}\n{SELECTORS}\n{MATCHED}\n{EXTRACT_BODY}\n}})()",
//     )
// }

/// The body of the DOM-extraction script, after the shared visibility/selector
/// fragments. Exposed so the constant can be assembled once at compile time.
pub const EXTRACT_BODY: &str = r"
    const elements = [];
    matched.slice(0, 100).forEach((el, i) => {
        const index = i + 1;
        const rect = el.getBoundingClientRect();
        const attrs = {};
        for (const attr of el.attributes) {
            attrs[attr.name] = attr.value;
        }

        let text = (el.innerText || el.textContent || el.getAttribute('aria-label') || el.getAttribute('placeholder') || el.getAttribute('value') || '').trim();
        if (text.length > 80) {
            text = text.substring(0, 77) + '...';
        }

        elements.push({
            index: index,
            tag: el.tagName.toLowerCase(),
            text: text,
            attributes: attrs,
            x: rect.left + rect.width / 2,
            y: rect.top + rect.height / 2
        });
    });

    let bodyText = document.body ? document.body.innerText : '';
    if (bodyText.length > 4000) {
        bodyText = bodyText.substring(0, 4000) + '...';
    }

    return {
        url: window.location.href,
        title: document.title,
        elements: elements,
        text: bodyText
    };
";

/// The complete DOM-extraction script, assembled lazily from the shared
/// fragments. `LazyLock` is used because `concat!()` only accepts string
/// literals, not `const` items, so the composite cannot be built purely at
/// compile time.
pub static EXTRACT_DOM_JS: LazyLock<String> = LazyLock::new(|| {
    format!("(() => {{\n{VISIBLE_FN}\n{SELECTORS}\n{MATCHED}\n{EXTRACT_BODY}\n}})()")
});

/// Builds the script used by [`Browser::get_element_info`] to fetch the tag,
/// input type, and bounding rect of the element at `index`.
#[must_use]
pub fn build_element_info_js(index: u32) -> String {
    format!(
        r"
        (() => {{
            {VISIBLE_FN}
            {SELECTORS}
            {MATCHED}

            const el = matched[{index} - 1];
            if (!el) return null;

            const rect = el.getBoundingClientRect();
            const tagName = el.tagName.toLowerCase();
            const type = el.getAttribute('type')?.toLowerCase() || '';

            return {{
                tag: tagName,
                type: type,
                x: rect.left,
                y: rect.top,
                width: rect.width,
                height: rect.height
            }};
        }})()
        ",
    )
}

/// Builds the script used by [`Browser::input`] to set the value of the element
/// at `index` to `text` (already JSON-escaped by the caller).
#[must_use]
pub fn build_input_js(index: u32, escaped_text: &str) -> String {
    format!(
        r"
        (() => {{
            {VISIBLE_FN}
            {SELECTORS}
            {MATCHED}

            const el = matched[{index} - 1];
            if (!el) return false;

            el.scrollIntoView({{ behavior: 'instant', block: 'center' }});

            const nativeInputValueSetter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, 'value').set;
            const nativeTextAreaValueSetter = Object.getOwnPropertyDescriptor(window.HTMLTextAreaElement.prototype, 'value').set;

            el.dispatchEvent(new FocusEvent('focus', {{ bubbles: true }}));

            if (el instanceof HTMLInputElement) {{
                nativeInputValueSetter.call(el, {escaped_text});
            }} else if (el instanceof HTMLTextAreaElement) {{
                nativeTextAreaValueSetter.call(el, {escaped_text});
            }} else if (el.isContentEditable) {{
                el.focus();
                el.textContent = '';
                el.textContent = {escaped_text};
            }} else {{
                nativeInputValueSetter.call(el, {escaped_text});
            }}

            const inputEvent = new Event('input', {{ bubbles: true, cancelable: true }});
            el.dispatchEvent(inputEvent);

            const changeEvent = new Event('change', {{ bubbles: true, cancelable: true }});
            el.dispatchEvent(changeEvent);

            el.dispatchEvent(new FocusEvent('blur', {{ bubbles: true }}));

            if (typeof jQuery !== 'undefined' && jQuery.fn) {{
                try {{ jQuery(el).trigger('change'); }} catch (e) {{}}
            }}

            return true;
        }})()
        ",
    )
}
