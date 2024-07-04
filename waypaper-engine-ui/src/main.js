const {invoke} = window.__TAURI__.core;
const {listen} = window.__TAURI__.event;

let greetInputEl;
let greetMsgEl;

function create(htmlStr) {
    let frag = document.createDocumentFragment(),
        temp = document.createElement('div');
    temp.innerHTML = htmlStr;

    while (temp.firstChild) {
        frag.appendChild(temp.firstChild);
    }

    return frag;
}

window.addEventListener("DOMContentLoaded", async () => {
    let grid = document.querySelector(".wp-grid");
    let screen_selector = document.querySelector(".screen-selector");
    let search_input = document.querySelector(".search-input");

    await listen('setWPs', (event) => {
        let frag = document.createDocumentFragment();
        let ids = [];

        for (const wp of event.payload) {
            frag.appendChild(create("<div class=\"wallpaper\" id='" + wp.id + "' style='background-image: url(" + wp.preview_b64 + ");'><h2>" + wp.title + "</h2></div>"));
            ids.push(wp.id);
        }

        grid.replaceChildren(frag);

        for (const id of ids) {
            document.getElementById(id).addEventListener("click", async (mouse_event) => {
                await invoke("set_wp", {wpId: id, screen: screen_selector.textContent})
            });
        }
    });

    search_input.addEventListener("input", async (event) => {
        await invoke("apply_filter", {search: event.target.value});
    });

    await invoke("loaded", {});
});
