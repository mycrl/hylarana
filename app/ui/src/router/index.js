import { createRouter, createWebHistory } from "vue-router";

import MainView from "@/views/Main.vue";

const router = createRouter({
    history: createWebHistory(import.meta.env.BASE_URL),
    routes: [
        {
            path: "/",
            name: "main",
            component: MainView,
        },
    ],
});

export default router;
