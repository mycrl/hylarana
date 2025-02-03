<template>
    <div id="Main">
        <MainHeader :type="type" @change="changeType" />
        <Sender v-if="type == 'sender'" />
        <Receiver v-if="type == 'receiver'" />
        <Settings v-if="type == 'settings'" />
    </div>
</template>

<script setup>
import MainHeader from "./header.vue";
import Settings from "./settings.vue";
import Sender from "./sender/index.vue";
import Receiver from "./receiver.vue";

import { ref } from "vue";

const type = ref("sender");

function changeType(kind) {
    type.value = kind;
}
</script>

<style scoped>
#Main {
    width: 1000px;
    height: 600px;
    position: relative;
    background-color: #fff;
}
</style>
