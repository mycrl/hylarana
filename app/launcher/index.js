"use strict";

require("electron")
    .app.whenReady()
    .then(() => require("./dist/main.js"));
