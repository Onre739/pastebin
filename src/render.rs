use axum::{response::Html, http::header, response::{IntoResponse, Response}};
use mermaid_svg::render;
use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd, html};
use syntect::html::highlighted_html_for_string;
use crate::model::{AppError, Paste, MimeKind};
use crate::store::{PasteStore, StyleStore};

const HOME_TEMPLATE: &str = include_str!("../templates/home.html");
//const PASTE_TEMPLATE: &str = include_str!("../templates/paste.html");

pub fn render_home_page(store: &PasteStore) -> Result<Html<String>, AppError> {

    let list_items: String = store.pastes.iter().map(|paste| {
        format!(r#"<li class="paste-item"><a href="/paste/{}"><span class="paste-name">{}</span><span class="paste-hits">{} hits</span></a></li>"#,
            paste.id, paste.name, paste.hits)
    }).collect::<Vec<String>>().join("\n");

    let list_items = if list_items.is_empty() {
        r#"<li class="paste-empty">No pastes yet.</li>"#.to_string()
    } else {
        list_items
    };

    let html_content = HOME_TEMPLATE.replace("{{PASTE_LIST}}", &list_items);
    Ok(Html(html_content))
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
                //Html(render::html_wrapper(paste.clone()))
            ).into_response()
        }

        MimeKind::Markdown => {

            let html_output = transform_md(&paste, &style_store)?;

            ([(header::CONTENT_TYPE, "text/html; charset=utf-8")],    
                Html(html_output)
            ).into_response()
        }

        MimeKind::OctetStream => {
            (
                [
                    (header::CONTENT_TYPE, "application/octet-stream"),
                    (header::CONTENT_DISPOSITION, &format!("attachment; filename=\"{}.bin\"", paste.id))
                ], paste.content.clone()
            ).into_response()
        }
        
    };

    Ok(response)
}

// fn html_wrapper(paste: Paste) -> String {
//     let content = String::from_utf8_lossy(&paste.content);

//     PASTE_TEMPLATE
//         .replace("{{NAME}}", &paste.name)
//         .replace("{{ID}}", &paste.id.to_string())
//         .replace("{{CONTENT}}", &content)
// }

fn transform_md (paste: &Paste, style_store: &StyleStore) -> Result<String, AppError> {
    
    // 1. Pulldown cmark
    let md_string = String::from_utf8(paste.content.clone()).map_err(|e| AppError::BadRequest(e))?;
    // map_err is needed for "?" operator, from_utf8 returns Result<String, FromUtf8Error> but ? needs AppError

    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_SMART_PUNCTUATION);
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_TASKLISTS);

    let theme = &style_store.theme_set.themes["Solarized (light)"];

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
                    new_events.push(Event::Html(html_content.into()));
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

