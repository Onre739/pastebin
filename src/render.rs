use crate::model;

const HOME_TEMPLATE: &str = include_str!("../templates/home.html");
const PASTE_TEMPLATE: &str = include_str!("../templates/paste.html");

pub fn render_homepage(pastes: Vec<model::Paste>) -> String {

    let list_items: String = pastes.iter().map(|paste| {
        format!(r#"<li class="paste-item"><a href="/paste/{}"><span class="paste-name">{}</span><span class="paste-hits">{} hits</span></a></li>"#,
            paste.id, paste.name, paste.hits)
    }).collect::<Vec<String>>().join("\n");

    let list_items = if list_items.is_empty() {
        r#"<li class="paste-empty">No pastes yet.</li>"#.to_string()
    } else {
        list_items
    };

    HOME_TEMPLATE.replace("{{PASTE_LIST}}", &list_items)
}

pub fn get_paste_by_uuid(paste: model::Paste) -> String {
    let content = String::from_utf8_lossy(&paste.content);

    PASTE_TEMPLATE
        .replace("{{NAME}}", &paste.name)
        .replace("{{ID}}", &paste.id.to_string())
        .replace("{{CONTENT}}", &content)
}