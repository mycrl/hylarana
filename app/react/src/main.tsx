import "./styles/index.css";

import Main from "./routes/main";
import { createRoot } from "react-dom/client";
import { BrowserRouter, Routes, Route } from "react-router";

createRoot(document.getElementById("root")!).render(
    <BrowserRouter>
        <Routes>
            <Route path='/' element={<Main />} />
        </Routes>
    </BrowserRouter>
);
