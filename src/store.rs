use std::sync::{Arc, Mutex};

use mermaid_svg::ast::Style;
use syntect::{highlighting::ThemeSet, parsing::SyntaxSet};

use crate::model::Paste;
#[derive(Clone)]
pub struct AppState {
    pub paste_store: Arc<Mutex<PasteStore>>,
    pub style_store: Arc<StyleStore>,
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

pub struct PasteStore {
    pub pastes: Vec<Paste>,
    pub max_pastes: usize,
}

impl PasteStore {
    
    pub fn process_full_capacity (&mut self) -> bool {
        let mut counter = 0;
        let len = self.pastes.len();
        
        loop {
            let index =  len - counter - 1;

            let paste = &mut self.pastes[index];
            paste.hits -= 1;

            if paste.hits <= 0 {
                self.pastes.remove(index);
                break true
            } 
    
            else {
                if (counter >= (len - 2)) { counter = 0; } // -2 because i dont want to decrese from new added paste (index 0)
                else { counter += 1; }
            }
        }

    }

    pub fn move_to_front (&mut self, index: usize) {
        if index < self.pastes.len() && index > 0 {
            let paste = self.pastes.remove(index);
            self.pastes.insert(0, paste);
        }
    }


}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

use super::*;

    #[test]
    fn idk(){
        
        
    }

}


