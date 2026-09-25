use axum::{response::Html, http::header, response::{IntoResponse, Response}};
use mermaid_svg::render;
use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd, html};
use syntect::html::highlighted_html_for_string;
use crate::model::{self, AppError, Paste, MimeKind};
use crate::store::StyleStore;

const HOME_TEMPLATE: &str = include_str!("../templates/home.html");
const MARKDOWN_TEMPLATE: &str = include_str!("../templates/markdown.html");
const OCTET_STREAM_TEMPLATE: &str = include_str!("../templates/octet_stream.html");

pub fn render_home_page(max_file_size: usize) -> Result<Html<String>, AppError> {
    let max_file_size_mb = max_file_size / (1024 * 1024);
    let page = HOME_TEMPLATE.replace("{{MAX_FILE_SIZE_MB}}", &max_file_size_mb.to_string());
    Ok(Html(page))
}

pub fn render_paste_page(paste: &Paste, style_store: &StyleStore) -> Result<Response, AppError> {
    let response = match paste.mimetype {

        MimeKind::PlainText => {
            ([(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
                paste.content.clone()
            ).into_response()
        }

        MimeKind::Html => {
            ([(header::CONTENT_TYPE, "text/html; charset=utf-8")],
                Html(paste.content.clone())
            ).into_response()
        }

        MimeKind::Markdown => {

            let html_output = transform_md(&paste, &style_store)?;
            let page = MARKDOWN_TEMPLATE.replace("{{CONTENT}}", &html_output);

            ([(header::CONTENT_TYPE, "text/html; charset=utf-8")],
                Html(page)
            ).into_response()
        }

        MimeKind::OctetStream => return render_octet_stream_page(paste),

    };

    Ok(response)
}

pub fn render_octet_stream_raw(paste: &Paste) -> Response {
    let filename = paste.file_name.clone().unwrap_or_else(|| format!("{}.bin", paste.id));
    (
        [
            (header::CONTENT_TYPE, "application/octet-stream"),
            (header::CONTENT_DISPOSITION, &format!("attachment; filename=\"{}\"", filename))
        ], paste.content.clone()
    ).into_response()
}

fn render_octet_stream_page(paste: &Paste) -> Result<Response, AppError> {
    let file_name = paste.file_name.clone().unwrap_or_else(|| format!("{}.bin", paste.id));
    let raw_url = format!("/paste/{}/raw", paste.id);

    let safe_file_name = html_escape(&file_name);

    let preview_html = if model::is_image_file_name(&file_name) {
        format!(r#"<img src="{}" alt="{}">"#, raw_url, safe_file_name)
    } else {
        String::from(r#"<p class="no-preview">No preview available for this file type.</p>"#)
    };

    let page = OCTET_STREAM_TEMPLATE
        .replace("{{FILE_NAME}}", &safe_file_name)
        .replace("{{FILE_SIZE}}", &format_file_size(paste.content.len()))
        .replace("{{RAW_URL}}", &raw_url)
        .replace("{{PREVIEW}}", &preview_html);

    Ok(Html(page).into_response())
}

fn html_escape(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn format_file_size(bytes: usize) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    let bytes_f = bytes as f64;

    if bytes_f >= MB {
        format!("{:.2} MB", bytes_f / MB)
    } else if bytes_f >= KB {
        format!("{:.1} KB", bytes_f / KB)
    } else {
        format!("{} B", bytes)
    }
}

fn transform_md (paste: &Paste, style_store: &StyleStore) -> Result<String, AppError> {
    
    // 1. Pulldown cmark
    let md_string = String::from_utf8(paste.content.clone()).map_err(|e| AppError::BadRequest(e.to_string()))?;
    // map_err is needed for "?" operator, from_utf8 returns Result<String, FromUtf8Error> but ? needs AppError

    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_SMART_PUNCTUATION);
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_TASKLISTS);

    let theme = &style_store.theme_set.themes["Solarized (dark)"];

    // 2. Parser
    let parser = Parser::new_ext(&md_string, options);
    let mut new_events = Vec::new();

    let mut in_code_block = false;
    let mut in_mermaid = false;
    let mut current_lang = String::new();
    let mut code_buffer = String::new();

    for event in parser {
        match event {
            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(lang))) => {  // ```lang
                if lang.as_ref() == "mermaid" {
                    in_mermaid = true;
                    code_buffer.clear();
                }
                else {
                    in_code_block = true;
                    current_lang = lang.to_string();
                    code_buffer.clear();
                }
            }

            Event::Text(t) if in_code_block || in_mermaid => {
                code_buffer.push_str(&t);
            }

            Event::End(TagEnd::CodeBlock) => {
                if in_mermaid {
                    in_mermaid = false;
                    
                    let html_content = render(&code_buffer).map_err(|e| AppError::MermaidRenderError(e))?;
                    let wrapped = format!(r#"<div class="mermaid-diagram">{}</div>"#, html_content);
                    new_events.push(Event::Html(wrapped.into()));
                }
                else if in_code_block {
                    in_code_block = false;
                    
                    let syntax = style_store.syntax_set.find_syntax_by_token(&current_lang)
                    .unwrap_or_else(|| style_store.syntax_set.find_syntax_plain_text());
    
                    let highlighted_html = highlighted_html_for_string(&code_buffer, &style_store.syntax_set, syntax, theme).map_err(|_| AppError::MarkdownParserFailed)?;
    
                    new_events.push(Event::Html(highlighted_html.into()));
                }            
            }

            other => {
                if !in_code_block {
                    new_events.push(other);
                }
            }
        }
    }

    let mut html_output = String::new();
    html::push_html(&mut html_output, new_events.into_iter());

    Ok(html_output)
}

#[cfg(test)]
mod tests {

    use uuid::Uuid;

use super::*;


    #[test]
    fn md_to_html() {
        let paste = Paste {
            id: Uuid::new_v4(),
            content: b"# Heading\n\nThis is **bold** and *italic* text.\n\n- item one\n- item two\n".to_vec(),
            mimetype: MimeKind::Markdown,
            hits: 0,
            last_seen_tick: 0,
            file_name: None,
        };

        let style_store = StyleStore::new();
        let html = transform_md(&paste, &style_store).expect("Markdown transform failed");

        assert!(html.contains("<h1>Heading</h1>"), "Heading not converted to <h1>");
        assert!(html.contains("<strong>bold</strong>"), "Bold text not converted to <strong>");
        assert!(html.contains("<em>italic</em>"), "Italic text not converted to <em>");
        assert!(html.contains("<li>item one</li>") && html.contains("<li>item two</li>"), "List items not converted to <li>");
    }

    #[test]
    fn highlight_code_block() {
        let paste = Paste {
            id: Uuid::new_v4(),
            content: b"```rust\nlet a = \"hello world\";\n```".to_vec(),
            mimetype: MimeKind::Markdown,
            hits: 0,
            last_seen_tick: 0,
            file_name: None,
        };

        let style_store = StyleStore::new();
        let html = transform_md(&paste, &style_store).expect("Markdown transform failed");

        assert!(html.contains("<pre") && html.contains("</pre>"), "Code block not found in HTML output");
        assert!(html.contains(r#"style="color:"#), "Syntax highlighting not found in HTML output");
    }

    #[test]
    fn mermaid() {
        let paste = Paste {
            id: Uuid::new_v4(),
            content: b"```mermaid\ngraph TD;\nA-->B;\nB-->C;\nC-->A;\n```".to_vec(),
            mimetype: MimeKind::Markdown,
            hits: 0,
            last_seen_tick: 0,
            file_name: None,
        };

        let style_store = StyleStore::new();
        let html = transform_md(&paste, &style_store).expect("Markdown transform failed");

        assert!(html.contains("<svg ") && html.contains("</svg>"), "Mermaid SVG not found in HTML output");
    }

    #[tokio::test]
    async fn plain_text() {
        let paste = Paste {
            id: Uuid::new_v4(),
            content: b"Hello, World!".to_vec(),
            mimetype: MimeKind::PlainText,
            hits: 0,
            last_seen_tick: 0,
            file_name: None,
        };

        let style_store = StyleStore::new();
        let response = render_paste_page(&paste, &style_store).expect("render_paste_page failed");

        let content_type = response.headers().get(header::CONTENT_TYPE).expect("missing Content-Type").to_str().unwrap();
        assert_eq!(content_type, "text/plain; charset=utf-8");

        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.expect("failed to read body");
        assert_eq!(body.as_ref(), paste.content.as_slice(), "PlainText body should pass through unchanged");
    }

    #[tokio::test]
    async fn octet_stream_raw() {
        let paste = Paste {
            id: Uuid::new_v4(),
            content: vec![0u8, 159, 146, 150, 1, 2, 3, 255],
            mimetype: MimeKind::OctetStream,
            hits: 0,
            last_seen_tick: 0,
            file_name: None,
        };

        let response = render_octet_stream_raw(&paste);

        let content_type = response.headers().get(header::CONTENT_TYPE).expect("missing Content-Type").to_str().unwrap();
        assert_eq!(content_type, "application/octet-stream");

        let content_disposition = response.headers().get(header::CONTENT_DISPOSITION).expect("missing Content-Disposition").to_str().unwrap();
        assert!(content_disposition.contains("attachment"), "Content-Disposition should mark the response as an attachment");
        assert!(content_disposition.contains(&paste.id.to_string()), "Content-Disposition filename should contain the paste id");

        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.expect("failed to read body");
        assert_eq!(body.as_ref(), paste.content.as_slice(), "OctetStream body should pass through unchanged");
    }

    #[tokio::test]
    async fn octet_stream_raw_with_file_name() {
        let paste = Paste {
            id: Uuid::new_v4(),
            content: vec![1, 2, 3, 4],
            mimetype: MimeKind::OctetStream,
            hits: 0,
            last_seen_tick: 0,
            file_name: Some("photo.png".to_string()),
        };

        let response = render_octet_stream_raw(&paste);

        let content_disposition = response.headers().get(header::CONTENT_DISPOSITION).expect("missing Content-Disposition").to_str().unwrap();
        assert_eq!(content_disposition, "attachment; filename=\"photo.png\"", "Content-Disposition should use the original file name when present");
    }

    #[tokio::test]
    async fn octet_stream_page_shows_image_preview() {
        let paste = Paste {
            id: Uuid::new_v4(),
            content: vec![1, 2, 3, 4],
            mimetype: MimeKind::OctetStream,
            hits: 0,
            last_seen_tick: 0,
            file_name: Some("photo.png".to_string()),
        };

        let style_store = StyleStore::new();
        let response = render_paste_page(&paste, &style_store).expect("render_paste_page failed");

        let content_type = response.headers().get(header::CONTENT_TYPE).expect("missing Content-Type").to_str().unwrap();
        assert_eq!(content_type, "text/html; charset=utf-8", "OctetStream page should be HTML, not the raw file");
        assert!(response.headers().get(header::CONTENT_DISPOSITION).is_none(), "the preview page itself should not force a download");

        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.expect("failed to read body");
        let body_str = String::from_utf8(body.to_vec()).unwrap();

        let raw_url = format!("/paste/{}/raw", paste.id);
        assert!(body_str.contains(&format!(r#"<img src="{}""#, raw_url)), "should embed an <img> pointing at the raw download URL");
        assert!(body_str.contains("photo.png"), "should show the file name");
        assert!(body_str.contains(&format!(r#"href="{}""#, raw_url)), "the download button should link to the raw URL");
    }

    #[tokio::test]
    async fn octet_stream_page_falls_back_to_no_preview() {
        let paste = Paste {
            id: Uuid::new_v4(),
            content: vec![1, 2, 3, 4],
            mimetype: MimeKind::OctetStream,
            hits: 0,
            last_seen_tick: 0,
            file_name: Some("archive.zip".to_string()),
        };

        let style_store = StyleStore::new();
        let response = render_paste_page(&paste, &style_store).expect("render_paste_page failed");

        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.expect("failed to read body");
        let body_str = String::from_utf8(body.to_vec()).unwrap();

        assert!(!body_str.contains("<img"), "non-image files should not get an <img> preview");
        assert!(body_str.contains("No preview available"), "should show the no-preview fallback message");
    }

    #[tokio::test]
    async fn octet_stream_page_escapes_file_name() {
        let paste = Paste {
            id: Uuid::new_v4(),
            content: vec![1, 2, 3, 4],
            mimetype: MimeKind::OctetStream,
            hits: 0,
            last_seen_tick: 0,
            file_name: Some("<script>alert(1)</script>.png".to_string()),
        };

        let style_store = StyleStore::new();
        let response = render_paste_page(&paste, &style_store).expect("render_paste_page failed");

        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.expect("failed to read body");
        let body_str = String::from_utf8(body.to_vec()).unwrap();

        assert!(!body_str.contains("<script>"), "file name must be HTML-escaped, not injected raw into the page");
        assert!(body_str.contains("&lt;script&gt;"), "escaped file name should still be visible as text");
    }
}