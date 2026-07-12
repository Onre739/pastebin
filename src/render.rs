use axum::response::{Html, IntoResponse};
use axum::extract::Path;

pub async fn render_homepage() -> impl IntoResponse {
    Html(r#"
    
    <div style="display:flex;align-items:center;justify-content:center;">
        <div>
            <h1>
                Pastebin by Onre
            </h1>

            <div style="display:flex;justify-content:center; gap: 50px;">
                <form id="paste-form">
                    <h3>Submit a Paste</h3>
                    <label for="content">Content:</label>
                    <input type="text" id="content" placeholder="Content"/>
                    
                    <br><br>

                    <label for="mimetype">MIME Type:</label>
                    <select id="mimetype">
                        <option value="text/plain">Plain Text</option>
                        <option value="text/html">HTML</option>
                        <option value="text/markdown">Markdown</option>
                        <option value="application/octet-stream">Octet Stream</option>
                    </select>

                    <br><br>

                    <button id="submit">Submit</button>
                
                </form>

                <div>
                    <h3>Paste Content</h3>
                    <ul id="paste-list">


                    </ul>
                </div>
            </div>
            
        </div>
    </div>

    <script>
        document.getElementById('submit').addEventListener('click', async (event) => {
            event.preventDefault();
            const content = document.getElementById('content').value;
            const mimetype = document.getElementById('mimetype').value;

            const response = await fetch('/paste', {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify({ content, mimetype }),
            });

            if (response.ok) {
                const data = await response.json();
                console.log('Paste submitted:', data);
                const pasteList = document.getElementById('paste-list');
                const listItem = document.createElement('li');
                listItem.textContent = `Paste ID: ${data.id}, Content: ${data.content}, MIME Type: ${data.mimetype}`;
                pasteList.appendChild(listItem);
            } else {
                console.error('Failed to submit paste');
            }
        });
    </script>

    "#)
}


pub async fn get_paste_by_uuid(Path(uuid): Path<String>) -> impl IntoResponse {
    
    let html_obsah = format!(r#"
        
            <div style="display:flex;align-items:center;justify-content:center;">
                <div>
                    <h1>
                        Pastebin by Onre
                    </h1>

                    <h3>Paste Content {uuid}</h3>
                    
                </div>
            </div>

        "#);

    Html(html_obsah)
}