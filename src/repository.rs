use axum::{
    response::{Html, IntoResponse, Response}, extract::State, http::{header, StatusCode},
};
use uuid::Uuid;
use std::{option, result, sync::{Arc, Mutex}};
use pulldown_cmark::{Event, Tag, TagEnd, CodeBlockKind, Parser, html, Options};
use syntect::{highlighting::ThemeSet, html::highlighted_html_for_string, parsing::SyntaxSet};
use mermaid_svg::render;

use crate::render;
use crate::store::PasteStore;
use crate::model::{MimeKind, Paste, AppError};

pub fn process_homepage (store: &PasteStore) -> Result<Html<String>, AppError> {   
    let html_content = render::render_homepage(store.pastes.clone());
    Ok(Html(html_content))
}

pub fn process_post (store: &mut PasteStore, name: String, content: String, mimetype: MimeKind) -> Result<Uuid, AppError> {
    let id = Uuid::new_v4();
    let paste = Paste {
        id,
        name: name,
        content: content.into_bytes(),
        mimetype: mimetype,
        hits: 0,
        last_seen_tick: 0
    };

    println!("Paste: {:#?}", paste);
    
    store.pastes.push(paste);
    Ok(id)
}

pub fn process_paste (store: &mut PasteStore, id: Uuid) -> Result<Response, AppError>{
    
    // 1. Find idex
    let index = store.pastes.iter().position(|p| p.id == id).ok_or(AppError::NotFound)?;
    
    // 2. Increment hits
    store.pastes[index].hits += 1;

    // 3. Get paste
    let paste = &store.pastes[index];
    
    let response = match paste.mimetype {
        MimeKind::PlainText => {
            ([(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
            paste.content.clone()
            ).into_response()
        }

        MimeKind::Html => {
            ([(header::CONTENT_TYPE, "text/html; charset=utf-8")],
                //Html(paste.content.clone())
                Html(render::get_paste_by_uuid(paste.clone()))

            ).into_response()
        }

        MimeKind::Markdown => {

            let html_output = transform_md(&paste, &store)?;

            ([(header::CONTENT_TYPE, "text/html; charset=utf-8")],    
                Html(html_output)
            ).into_response()
        }

        MimeKind::OctetStream => {
            (
                [
                    (header::CONTENT_TYPE, "application/octet-stream"),
                    (header::CONTENT_DISPOSITION, &format!("attachment; filename=\"{}.bin\"", id))
                ], paste.content.clone()
            ).into_response()
        }
    };

    // 3. Switch place to front of vector
    store.move_to_front(index);

    // 4. Check max pastes
    let is_full = store.pastes.len() > store.max_pastes;

    if is_full {
        store.process_full_capacity();
    }

    Ok(response)        
}

fn transform_md (paste: &Paste, store: &PasteStore) -> Result<String, AppError> {
    
    // 1. Pulldown cmark
    let md_string = String::from_utf8(paste.content.clone()).map_err(|e| AppError::BadRequest(e))?;
    // map_err is needed for "?" operator, from_utf8 returns Result<String, FromUtf8Error> but ? needs AppError

    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_SMART_PUNCTUATION);
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_TASKLISTS);

    let theme = &store.theme_set.themes["Solarized (light)"];

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
                    
                    let syntax = store.syntax_set.find_syntax_by_token(&current_lang)
                    .unwrap_or_else(|| store.syntax_set.find_syntax_plain_text());
    
                    let highlighted_html = highlighted_html_for_string(&code_buffer, &store.syntax_set, syntax, theme).map_err(|_| AppError::MarkdownParserFailed)?;
    
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