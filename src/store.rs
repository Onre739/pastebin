use std::sync::{Arc, Mutex};

use mermaid_svg::ast::Style;
use syntect::{highlighting::ThemeSet, parsing::SyntaxSet};
use uuid::Uuid;

use crate::model::{AppError, Paste};
#[derive(Clone)]
pub struct AppState {
    pub paste_store: Arc<Mutex<PasteStore>>,
    pub style_store: Arc<StyleStore>,
}

pub struct PasteStore {
    pub pastes: Vec<Paste>,
    pub max_pastes: usize,
    pub current_tick: u64,
}

pub struct StyleStore {
    pub syntax_set: SyntaxSet,
    pub theme_set: ThemeSet,
}

impl StyleStore {
    pub fn new() -> Self {
        Self { 
            syntax_set: SyntaxSet::load_defaults_newlines(), 
            theme_set: ThemeSet::load_defaults() 
        }
    }
}

impl PasteStore {

    pub fn new (max_pastes: usize) -> Self {
        PasteStore { pastes: Vec::new(), max_pastes, current_tick: 0 }
    }

    pub fn increment_hits ( paste: &mut Paste) {
        paste.hits += 1;
    }

    pub fn decrement_hits ( paste: &mut Paste) {
        if paste.hits > 0 {
            paste.hits -= 1;
        }
    }

    pub fn next_tick (&mut self) -> u64 {
        self.current_tick += 1;
        self.current_tick
    }

    pub fn insert_paste (&mut self, mut paste: Paste) {
        paste.last_seen_tick = self.next_tick();
        self.pastes.push(paste);
    }
    
    pub fn check_full_capacity(&self) -> bool {
        self.pastes.len() >= self.max_pastes
    }

    pub fn find_last_seen_paste(&self, pastes: &Vec<Paste>, indexes_to_skip : &Vec<usize>) -> Option<usize> {
        let mut i = 0;

        // Initialize last_seen_paste_index to the first index that is not in indexes_to_skip
        let mut last_seen_paste_index = {
            let mut f = 0;
            loop {
                if !indexes_to_skip.iter().any(|x| *x == f) {
                    break f;
                }

                if f >= pastes.len() - 1 {
                    return None;
                }

                f += 1;
            }
        };


        for p in pastes {
            
            let skip = indexes_to_skip.iter().any(|x| *x == i);
            
            if !skip && p.last_seen_tick <= pastes[last_seen_paste_index].last_seen_tick {
                last_seen_paste_index = i;
            }
            i += 1;
        }
        Some(last_seen_paste_index)
    }

    pub fn process_full_capacity (&mut self) -> Result<bool, AppError> {
        let mut indexes_to_skip: Vec<usize> = Vec::new();
        
        loop {
            let last_seen_paste_index = self.find_last_seen_paste(&self.pastes, &indexes_to_skip).ok_or(AppError::FullCapacityEvictionFailed)?;
            let last_paste = &mut self.pastes[last_seen_paste_index];
            Self::decrement_hits(last_paste);

            if last_paste.hits == 0 {
                self.pastes.remove(last_seen_paste_index);
                break Ok(true)
            }
            else {
                indexes_to_skip.push(last_seen_paste_index);

                if indexes_to_skip.len() >= self.pastes.len() {
                    indexes_to_skip.clear();
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex, MutexGuard};
    use uuid::Uuid;

use crate::model::MimeKind;

use super::*;

    fn create_empty_store (max_pastes: usize) -> Arc<Mutex<PasteStore>> {
        Arc::new(Mutex::new(PasteStore::new(max_pastes)))
        //let paste_store = store_guard.lock().unwrap();
        //paste_store
    }

    fn create_test_paste () -> Paste {
        let id = Uuid::new_v4();
        Paste {
            id,
            name: format!("Pepa - {}", id),
            content: Vec::new(),
            mimetype: MimeKind::PlainText,
            hits: 0,
            last_seen_tick: 0
        }
    }

    #[test]
    fn insert_paste(){
        let store_arc = create_empty_store(5);
        let mut paste_store = store_arc.lock().unwrap();
        
        let paste1 = create_test_paste();
        paste_store.insert_paste(paste1);

        assert_eq!(paste_store.pastes.len(), 1);
    }

    #[test]
    fn insert_limit(){
        let store_arc = create_empty_store(5);
        let mut paste_store = store_arc.lock().unwrap();
        for i in 0..5 {
            let paste = create_test_paste();
            paste_store.insert_paste(paste);
        }

        assert_eq!(paste_store.pastes.len(), 5);
    }

    
}


