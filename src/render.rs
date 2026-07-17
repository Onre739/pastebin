use std::fmt::format;
use axum::response::{Html, IntoResponse};

use crate::model;

pub fn render_homepage(pastes: Vec<model::Paste>) -> impl IntoResponse {
    
    let mut list_items = Vec::new();

    for paste in &pastes {
        let preview_content = String::from_utf8_lossy(&paste.content);

        let list_item = if preview_content.chars().count() > 20 {
            format!(r#"<li><a href="/paste/{}">{}...</a></li>"#, paste.id, &preview_content[..20])
        }
        else {
            format!(r#"<li><a href="/paste/{}">{}</a></li>"#, paste.id, preview_content)
        };

        list_items.push(list_item);
    }

    let list_items = list_items.join("\n");
    
    
    // let list_items = pastes.iter().map(|paste|{

    //     let preview_content = String::from_utf8_lossy(&paste.content); 

    //     if preview_content.chars().count() > 20 {
    //         format!(r#"<li><a href="/paste/{}">{}...</a></li>"#, paste.id, &preview_content[..20])
    //     }
    //     else {
    //         format!(r#"<li><a href="/paste/{}">{}</a></li>"#, paste.id, preview_content)
    //     }

    // })
    // .collect::<Vec<String>>()
    // .join("\n");
    
    
    let html_content = format!(r#"
    <div style="display:flex;align-items:center;justify-content:center;">
        <div>
            <h1>Pastebin by Onre</h1>

            <div style="display:flex;justify-content:center; gap: 50px;">
                <form id="paste-form">
                    <h3>Submit a Paste</h3>
                    <label for="content">Content:</label>
                    <input type="text" id="content" placeholder="Content"/>
                    
                    <br><br>

                    <label for="mimetype">MIME Type:</label>
                    <select id="mimetype">
                        <option value="PlainText">Plain Text</option>
                        <option value="Html">HTML</option>
                        <option value="Markdown">Markdown</option>
                        <option value="OctetStream">Octet Stream</option>
                    </select>

                    <br><br>

                    <button id="submit">Submit</button>
                </form>

                <div>
                    <h3>Paste Content</h3>
                    <ul id="paste-list">
                        {list_items}
                    </ul>
                </div>
            </div>
        </div>
    </div>

    <script>

        let ul = document.getElementById('paste-list');
        

        document.getElementById('submit').addEventListener('click', async (event) => {{
            event.preventDefault();
            const contentValue = document.getElementById('content').value;
            const mimetypeValue = document.getElementById('mimetype').value;

            const response = await fetch('/paste/json', {{
                method: 'POST',
                headers: {{
                    'Content-Type': 'application/json',
                }},
                body: JSON.stringify({{ "content": contentValue, "mimetype": mimetypeValue }}),
            }});

            if (response.redirected) {{
                window.location.href = response.url;
            }} else {{
                console.error('Failed to submit paste');
            }}
        }});
    </script>
    "#);

    Html(html_content)
}


pub fn get_paste_by_uuid(paste: model::Paste) -> impl IntoResponse + use<> {
    let id = paste.id;
    let content = String::from_utf8_lossy(&paste.content);
    
    
    let html_obsah = format!(r#"
        
            <div style="display:flex;align-items:center;justify-content:center;">
                <div>
                    <div style="display:flex;align-items:center;justify-content:center; gap: 50px;">
                        <h1>
                            Pastebin by Onre
                        </h1>
                        <button onclick="window.location.href='/'">Back to Home</button>
                    </div>

                    <h3>Paste Content of {id}</h3>
                    <p>{content}</p>
                    
                </div>
            </div>

        "#);

    Html(html_obsah)
}