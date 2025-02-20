const { contextBridge, ipcRenderer } = require("electron/renderer");

contextBridge.exposeInMainWorld("Route", {
    request: (method, content) => ipcRenderer.invoke(method, content),
    on: (method, listener) =>
        ipcRenderer.on(method, (_, content) => {
            listener(content);
        }),
});
