import "./styles/index.css";

import Main from "./app";
import { createRoot } from "react-dom/client";
import { Suspense } from "react";

createRoot(document.getElementById("root")!).render(
    <Suspense>
        <Main />
    </Suspense>
);
