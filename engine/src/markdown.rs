use pulldown_cmark::{html, Options, Parser};

/// Converts Markdown to a minimal standalone HTML document (CommonMark plus
/// GFM tables/strikethrough/footnotes). The caller renders the HTML to PDF
/// separately (Android's WebView print pipeline) — this function only does
/// the text transform, no PDF/layout work.
#[uniffi::export]
pub fn convert_markdown_to_html(markdown: String) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_TASKLISTS);

    let parser = Parser::new_ext(&markdown, options);
    let mut body = String::new();
    html::push_html(&mut body, parser);

    format!("<!DOCTYPE html><html><head><meta charset=\"utf-8\"></head><body>{body}</body></html>")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_heading_and_paragraph() {
        let html = convert_markdown_to_html("# Title\n\nSome *text*.".to_string());
        assert!(html.contains("<h1>Title</h1>"));
        assert!(html.contains("<em>text</em>"));
    }

    #[test]
    fn renders_gfm_table() {
        let html = convert_markdown_to_html("| a | b |\n|---|---|\n| 1 | 2 |".to_string());
        assert!(html.contains("<table>"));
    }
}
