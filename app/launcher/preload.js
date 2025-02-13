const { contextBridge, ipcRenderer } = require("electron/renderer");

contextBridge.exposeInMainWorld("MessageTransport", {
    send: (message) => ipcRenderer.send("MessageTransport", message),
    on: (listener) => {
        ipcRenderer.on("MessageTransport", (_, message) => {
            listener(message);
        });
    },
    setName: (name) => ipcRenderer.invoke("SetName", name),
    getName: () => ipcRenderer.invoke("GetName"),
});
