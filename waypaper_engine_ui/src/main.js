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
    let stop_daemon_btn = document.querySelector(".stop-daemon-btn");

    let screens = await invoke("get_screens", {});
    console.dir(screens);
    let frag = document.createDocumentFragment();
    for (const screen of screens) {
        frag.appendChild(create("<option>" + screen + "<option>"));
    }
    screen_selector.replaceChildren(frag);

    screen_selector.querySelector("option:empty").remove();

    await listen('setWPs', (event) => {
        let frag = document.createDocumentFragment();
        let ids = [];

        for (const wp of event.payload) {
            frag.appendChild(create("<div class=\"wallpaper\" id='" + wp.id + "' style='background-image: url(" + wp.preview_b64 + ");'><h2>" + wp.title + "</h2></div>"));
            ids.push(wp.id);
        }

        grid.replaceChildren(frag);

        for (const id of ids) {
            document.getElementById(id).addEventListener("click", async (/*mouse_event*/) => {
                console.dir(screen_selector);
                await invoke("set_wp", {wpId: id, screen: screen_selector.value})
            });
        }
    });

    search_input.addEventListener("input", async (event) => {
        await invoke("apply_filter", {search: event.target.value});
    });
    
    stop_daemon_btn.addEventListener("click", async (event) => {
        await invoke("stop_daemon", {});
    });

    await invoke("loaded", {});
});
