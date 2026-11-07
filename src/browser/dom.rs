use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedElement {
    pub index: u32,
    pub tag: String,
    pub text: String,
    pub attributes: HashMap<String, String>,
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserState {
    pub url: String,
    pub title: String,
    pub elements: Vec<EnhancedElement>,
    pub text: String,
}

impl BrowserState {
    pub fn format_for_prompt(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("Current URL: {}\n", self.url));
        out.push_str(&format!("Page Title: {}\n", self.title));

        out.push_str("\nInteractive Elements:\n");
        if self.elements.is_empty() {
            out.push_str("(No interactive elements found)\n");
        } else {
            for el in &self.elements {
                let mut attr_str = String::new();
                for (k, v) in &el.attributes {
                    if [
                        "type",
                        "name",
                        "id",
                        "placeholder",
                        "aria-label",
                        "href",
                        "role",
                    ]
                    .contains(&k.as_str())
                    {
                        attr_str.push_str(&format!(" {k}=\"{v}\""));
                    }
                }
                out.push_str(&format!(
                    "[{}]{}{}>{}</{}>\n",
                    el.index, el.tag, attr_str, el.text, el.tag
                ));
            }
        }

        out.push_str("\nPage Text Content (summary):\n");
        let truncated_text = if self.text.len() > 3000 {
            format!("{}...", &self.text[..3000])
        } else {
            self.text.clone()
        };
        out.push_str(&truncated_text);
        out.push('\n');

        out
    }
}

pub const EXTRACT_DOM_JS: &str = r#"
(() => {
    const isVisible = (elem) => {
        if (!elem) return false;
        const style = window.getComputedStyle(elem);
        if (style.display === 'none' || style.visibility === 'hidden' || style.opacity === '0') return false;
        const rect = elem.getBoundingClientRect();
        return rect.width > 0 && rect.height > 0 && rect.bottom >= 0 && rect.top <= window.innerHeight;
    };

    const interactiveSelectors = [
        'a[href]', 'button', 'input', 'textarea', 'select',
        '[role="button"]', '[role="link"]', '[role="checkbox"]', '[role="menuitem"]',
        '[role="tab"]', '[role="combobox"]', '[role="switch"]', '[tabindex]:not([tabindex="-1"])'
    ];

    const elements = [];
    const matched = Array.from(document.querySelectorAll(interactiveSelectors.join(',')))
        .filter(isVisible);

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
})()
"#;
