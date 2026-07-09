use axum::response::{Html, IntoResponse};

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

    "#)
}