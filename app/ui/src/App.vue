<script setup>
import { RouterView } from "vue-router";
import { Language, setLanguage } from "@/i18n";

if (!Language.value) {
    setLanguage("english");
}
</script>

<template>
    <RouterView />
</template>
