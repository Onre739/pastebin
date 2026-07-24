use axum::{
    response::{Html, IntoResponse, Response}, extract::State, http::{header, StatusCode},
};
use uuid::Uuid;
use std::{option, result, sync::{Arc, Mutex}};
use pulldown_cmark::{Parser, html, Options};

use crate::render;
use crate::store::PasteStore;
use crate::model::{MimeKind, Paste, AppError};


pub fn process_homepage (store: &PasteStore) -> Html<String> {   
    let html_content = render::render_homepage(store.pastes.clone());
    Html(html_content)
}

pub fn process_post (store: &mut PasteStore, name: String, content: String, mimetype: MimeKind) -> Uuid {
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
    id
}

pub fn process_paste (store: &mut PasteStore, id: Uuid) -> Result<Response, AppError>{
    
    // 1. Find idex
    let index = store.pastes.iter().position(|p| p.id == id).ok_or(AppError::NotFound)?;
    
    // 2. Get paste
    let paste = &mut store.pastes[index];
    paste.hits += 1;
    
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
            let md_string = String::from_utf8(paste.content.clone());
            let mut html_output = String::new();

            let mut options = Options::empty();
            options.insert(Options::ENABLE_STRIKETHROUGH);
            options.insert(Options::ENABLE_SMART_PUNCTUATION);
            options.insert(Options::ENABLE_TABLES);
            options.insert(Options::ENABLE_TASKLISTS);

            match md_string {
                Ok(md_string_correct) => {
                    let parser = Parser::new_ext(&md_string_correct, options);
                    html::push_html(&mut html_output, parser);
                }
                Err(e) => {
                    html_output = format!("<p>Error converting Markdown to HTML: {}</p>", e);
                }
            }

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