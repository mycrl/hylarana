import { createRouter, createWebHistory } from "vue-router";

import MainView from "@/views/Main.vue";

export default createRouter({
    history: createWebHistory(import.meta.env.BASE_URL),
    routes: [
        {
            path: "/",
            name: "main",
            component: MainView,
        },
    ],
});
