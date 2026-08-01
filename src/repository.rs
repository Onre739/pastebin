use axum::{
    response::{Html, IntoResponse, Response}, extract::State, http::{header, StatusCode},
};
use uuid::Uuid;
use std::{option, result, sync::{Arc, Mutex}};
use pulldown_cmark::{Event, Tag, TagEnd, CodeBlockKind, Parser, html, Options};
use syntect::{highlighting::ThemeSet, html::highlighted_html_for_string, parsing::SyntaxSet};
use mermaid_svg::render;

use crate::{render, store::StyleStore};
use crate::store::PasteStore;
use crate::model::{MimeKind, Paste, AppError};

pub fn process_homepage (store: &PasteStore) -> Result<Html<String>, AppError> {   
    let html_content = render::render_homepage(store.pastes.clone());
    Ok(Html(html_content))
}

pub fn process_post (paste_store: &mut PasteStore, name: String, content: String, mimetype: MimeKind) -> Result<Uuid, AppError> {
    let id = Uuid::new_v4();
    let paste = Paste {
        id,
        name: name,
        content: content.into_bytes(),
        mimetype: mimetype,
        hits: 0,
        last_seen_tick: 0
    };

    if paste_store.check_full_capacity() {
        paste_store.process_full_capacity()?;
    }

    paste_store.insert_paste(paste);
    
    Ok(id)
}

pub fn process_paste (store: &mut PasteStore, style_store: &StyleStore, id: Uuid) -> Result<Response, AppError>{
    
    // 1. Find idex
    let index = store.pastes.iter().position(|p| p.id == id).ok_or(AppError::NotFound)?;
    
    // 2. Increment hit & tick
    let tick = store.get_next_tick();
    store.pastes[index].update(tick);

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

            let html_output = transform_md(&paste, &style_store)?;

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

    Ok(response)        
}

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

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};
    use uuid::Uuid;

use crate::model::MimeKind;
use super::*;

    fn create_empty_store (max_pastes: usize) -> Arc<Mutex<PasteStore>> {
        Arc::new(Mutex::new(PasteStore::new(max_pastes)))
    }

    #[test]
    fn insert_max () {
        let store_arc = create_empty_store(5);
        let mut paste_store = store_arc.lock().unwrap();
        let mut first_id = Uuid::nil();

        for i in 0..6 {
            process_post(&mut paste_store, format!("Paste {}", i), format!("Content {}", i), MimeKind::PlainText);
            if i == 0 { first_id = paste_store.pastes[0].id; }
        }

        assert_eq!(paste_store.pastes.len(), 5);
        assert!(!paste_store.pastes.iter().any(|p| p.id == first_id), "First paste should have been evicted");
    }

    #[test]
    fn decrement_cyclus () {
        let store_arc = create_empty_store(5);
        let mut paste_store = store_arc.lock().unwrap();
        let mut second_id = Uuid::nil();

        for i in 0..6 {
            process_post(&mut paste_store, format!("Paste {}", i), format!("Content {}", i), MimeKind::PlainText);
            if i == 1 { 
                second_id = paste_store.pastes[1].id; 
            }
            else if i == 0 {
                paste_store.pastes[0].increment_hits();
                paste_store.pastes[0].increment_hits();
                println!("Incremented hits for paste 0: {}", paste_store.pastes[0].hits);
            }
        }

        assert_eq!(paste_store.pastes.len(), 5);
        assert!(!paste_store.pastes.iter().any(|p| p.id == second_id), "Second paste should have been evicted");
    }

    #[test]
    fn hits_and_ticks_increment () {
        let store_arc = create_empty_store(5);
        let mut paste_store = store_arc.lock().unwrap();
        let style_store = StyleStore::new();

        process_post(&mut paste_store, format!("Paste {}", 0), format!("Content {}", 0), MimeKind::PlainText);
        
        let id = paste_store.pastes[0].id;
        process_paste(&mut paste_store, &style_store, id).expect("Paste processing failed");

        assert_eq!(paste_store.pastes[0].hits, 1, "Hits should have been incremented");
        assert_eq!(paste_store.pastes[0].last_seen_tick, 2, "Last seen tick should have been incremented to 2 (1 for insert, 1 for access)");
    }

    #[test]
    fn same_tick () {
        let store_arc = create_empty_store(5);
        let mut paste_store = store_arc.lock().unwrap();

        for i in 0..6 {
            if i == 0 { 
                paste_store.insert_paste(Paste {
                    id: Uuid::new_v4(),
                    name: format!("Paste {}", i),
                    content: format!("Content {}", i).into_bytes(),
                    mimetype: MimeKind::PlainText,
                    hits: 0,
                    last_seen_tick: 0
                });
            }
            else{
                process_post(&mut paste_store, format!("Paste {}", i), format!("Content {}", i), MimeKind::PlainText);
            }
        }

        println!("Index 0 tick: {}, Index 1 tick: {}", paste_store.pastes[0].last_seen_tick, paste_store.pastes[1].last_seen_tick);
        assert_eq!(paste_store.pastes.len(), 5);
    }

    #[test]
    fn max_paste_1 () {
        let store_arc = create_empty_store(1);
        let mut paste_store = store_arc.lock().unwrap();

        process_post(&mut paste_store, format!("Paste {}", 0), format!("Content {}", 0), MimeKind::PlainText);
        process_post(&mut paste_store, format!("Paste {}", 1), format!("Content {}", 1), MimeKind::PlainText);

        assert_eq!(paste_store.pastes.len(), 1);
    }

    #[test]
    #[should_panic]
    fn empty_store () {
        let store_arc = create_empty_store(5);
        let mut paste_store = store_arc.lock().unwrap();
        let random_id = Uuid::new_v4();
        let style_store = StyleStore::new();

        process_paste(&mut paste_store, &style_store, random_id);

        panic!("Should have returned NotFound error");
    }

}