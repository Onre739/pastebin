use axum::{
    Json, Router, extract::{Path,State, Form}, http::{StatusCode, header}, response::{Html, IntoResponse, Redirect}, routing::{get, post},
};
use std::{option, sync::{Arc, Mutex}};
use uuid::Uuid;
use pulldown_cmark::{Parser, html, Options};
use crate::model;
use crate::render;
use crate::store::PasteStore;

pub fn create_router(state: Arc<Mutex<PasteStore>>) -> Router {
    let app = Router::new()
        .route("/", get(get_home))
        .route("/paste/json", post(post_paste_json))
        .route("/paste/form", post(post_paste_form))
        .route("/paste/{uuid}", get(get_paste))
        .with_state(state);
    app
}

async fn get_home(State(state): State<Arc<Mutex<PasteStore>>>) -> impl IntoResponse {
    
    let store = state.lock().unwrap();
    
    let html_content = render::render_homepage(store.pastes.clone());
    Html(html_content)
}

async fn post_paste_json(State(state): State<Arc<Mutex<PasteStore>>>, Json(payload): Json<model::CreatePasteDto>) 
-> impl IntoResponse {
    
    let id = Uuid::new_v4();
    let paste = model::Paste {
        id,
        content: payload.content.into_bytes(),
        mimetype: payload.mimetype,
        hits: 0,
        last_seen_tick: 0
    };

    println!("Paste: {:#?}", paste);
    
    let mut store = state.lock().unwrap();
    store.pastes.push(paste);
    
    Redirect::to(&format!("/paste/{}", id))
}

async fn post_paste_form(State(state): State<Arc<Mutex<PasteStore>>>, Form(form): Form<model::CreatePasteDto>) -> impl IntoResponse {
    
    let id = Uuid::new_v4();
    let paste = model::Paste {
        id,
        content: form.content.into_bytes(),
        mimetype: form.mimetype,
        hits: 0,
        last_seen_tick: 0
    };

    println!("Paste: {:#?}", paste);
    
    let mut store = state.lock().unwrap();
    store.pastes.push(paste);
    
    Redirect::to(&format!("/paste/{}", id))
}

async fn get_paste(Path(uuid): Path<Uuid>, State(state): State<Arc<Mutex<PasteStore>>>) -> impl IntoResponse {
    println!("Looking for paste with UUID: {}", uuid);

    let store = state.lock().unwrap();
    let paste = store.pastes.iter().find(|p| p.id == uuid);
    
    // Ok & Err not working here, .into_response() works fine
    match paste {
        Some(paste) => {
            println!("Mimetype: {:#?}", paste.mimetype);
            
            match paste.mimetype {
                model::MimeKind::PlainText => {
                    ([(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
                    paste.content.clone()
                    ).into_response()
                }

                model::MimeKind::Html => {
                    ([(header::CONTENT_TYPE, "text/html; charset=utf-8")],
                        render::get_paste_by_uuid(paste.clone())
                    ).into_response()
                }

                model::MimeKind::Markdown => {
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

                model::MimeKind::OctetStream => {
                    (
                        [
                            (header::CONTENT_TYPE, "application/octet-stream"),
                            (header::CONTENT_DISPOSITION, &format!("attachment; filename=\"{}.bin\"", uuid))
                        ], paste.content.clone()
                    ).into_response()
                }
            }
            
        },
        
        None => {
            StatusCode::NOT_FOUND.into_response()
        }
    }

}
