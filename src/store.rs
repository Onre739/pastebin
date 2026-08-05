use std::sync::{Arc, Mutex};
use syntect::{highlighting::ThemeSet, parsing::SyntaxSet};
use uuid::Uuid;

use crate::model::{AppError, MimeKind, Paste};
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

    pub fn get_next_tick (&mut self) -> u64 {
        self.current_tick += 1;
        self.current_tick
    }

    pub fn insert (&mut self, name: String, content: Vec<u8>, mimetype: MimeKind) -> Result<Uuid, AppError> {
        if self.check_full_capacity() {
            self.process_full_capacity()?;
        }   

        let id = Uuid::new_v4();
        let paste = Paste {
            id,
            name: name,
            content: content,
            mimetype: mimetype,
            hits: 0,
            last_seen_tick: self.get_next_tick()
        };

        self.pastes.push(paste);
        Ok(id)
    }

    pub fn record_view (&mut self, uuid: Uuid) -> Result<&Paste, AppError> {
        let index = self.pastes.iter().position(|p| p.id == uuid).ok_or(AppError::NotFound)?;
        
        let tick = self.get_next_tick();
        self.pastes[index].update(tick);
        
        Ok(&self.pastes[index])
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
        if self.pastes.is_empty() {
            return Err(AppError::NotFound);
        }
        
        let mut indexes_to_skip: Vec<usize> = Vec::new();
        
        loop {
            let last_seen_paste_index = self.find_last_seen_paste(&self.pastes, &indexes_to_skip).ok_or(AppError::FullCapacityEvictionFailed)?;
            let last_paste = &mut self.pastes[last_seen_paste_index];
            last_paste.decrement_hits();

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
    use std::sync::{Arc, Mutex};
    use uuid::Uuid;

use crate::model::MimeKind;
use super::*;
    fn create_empty_store (max_pastes: usize) -> PasteStore {
        PasteStore::new(max_pastes)
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
    fn insert(){
        let mut paste_store = create_empty_store(5);
        
        let paste = create_test_paste();
        paste_store.insert(paste.name, paste.content, paste.mimetype).expect("Insert failed");

        assert_eq!(paste_store.pastes.len(), 1);
    }

    #[test]
    fn insert_limit(){
        let mut paste_store = create_empty_store(5);
        for i in 0..5 {
            let paste = create_test_paste();
            paste_store.insert(format!("Paste {}", i), paste.content, paste.mimetype).expect("Insert failed");
        }

        assert_eq!(paste_store.pastes.len(), 5);
    }

    #[test]
    fn insert_max () {
        let mut paste_store = create_empty_store(5);
        let mut first_id = Uuid::nil();
    
        for i in 0..6 {
            let paste = create_test_paste();
            paste_store.insert(format!("Paste {}", i), paste.content, paste.mimetype).expect("Insert failed");
            if i == 0 { first_id = paste_store.pastes[0].id; }
        }
    
        assert_eq!(paste_store.pastes.len(), 5);
        assert!(!paste_store.pastes.iter().any(|p| p.id == first_id), "First paste should have been evicted");
    }
    
    #[test]
    fn decrement_cyclus () {
        let mut paste_store = create_empty_store(5);
        let mut second_id = Uuid::nil();
    
        for i in 0..6 {
            let paste = create_test_paste();
            paste_store.insert(format!("Paste {}", i), paste.content, paste.mimetype).expect("Insert failed");
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
        let mut paste_store = create_empty_store(5);
        let paste = create_test_paste();

        paste_store.insert(format!("Paste {}", 0), paste.content, paste.mimetype).expect("Insert failed");
        
        let uuid = paste_store.pastes[0].id;
        paste_store.record_view(uuid).expect("Should have increment hits and ticks & return paste");

        assert_eq!(paste_store.pastes[0].hits, 1, "Hits should have been incremented");
        assert_eq!(paste_store.pastes[0].last_seen_tick, 2, "Last seen tick should have been incremented to 2 (1 for insert, 1 for access)");
    }
    
    #[test]
    fn same_tick () {
        let mut paste_store = create_empty_store(5);

        let pastes = vec![
            create_test_paste(),
            create_test_paste(),
            create_test_paste(),
        ];

        let index = paste_store.find_last_seen_paste(&pastes, &Vec::new()).expect("Should have found a last seen paste");
    
        println!("Index 0 tick: {}, Index 1 tick: {}", pastes[0].last_seen_tick, pastes[1].last_seen_tick);
        assert_eq!(index, 2, "Should have returned the last index when all ticks are the same");
    }
    
    #[test]
    fn max_paste_1 () {
        let mut paste_store = create_empty_store(1);
    
        paste_store.insert(format!("Paste {}", 0), format!("Content {}", 0).into_bytes(), MimeKind::PlainText).expect("Insert failed");
        paste_store.insert(format!("Paste {}", 1), format!("Content {}", 1).into_bytes(), MimeKind::PlainText).expect("Insert failed");
    
        assert_eq!(paste_store.pastes.len(), 1);
    }
    
    #[test]
    fn empty_store () {
        let mut paste_store = create_empty_store(5);
    
        let result = paste_store.process_full_capacity();
        assert!(matches!(result, Err(AppError::NotFound)), "Should have returned NotFound error");
    }
}