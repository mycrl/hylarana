import "./styles/index.css";

import Main from "./routes/main";
import { createRoot } from "react-dom/client";
import { BrowserRouter, Routes, Route } from "react-router";
import { Suspense } from "react";

createRoot(document.getElementById("root")!).render(
    <Suspense>
        <BrowserRouter>
            <Routes>
                <Route path='/' element={<Main />} />
            </Routes>
        </BrowserRouter>
    </Suspense>
);
