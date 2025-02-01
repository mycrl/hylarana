<template>
    <div id="Sender">
        <div id="switch">
            <div class="body">
                <span>{{ I18n.Broadcast }}</span>
                <Switch :value="false" />
            </div>
            <p>{{ I18n.BroadcastTips }}</p>
        </div>
    </div>
</template>

<script setup>
import { I18n } from "@/i18n";

import Switch from "@/components/Switch.vue";
</script>

<style scoped>
#Sender {
    position: absolute;
    width: 100%;
    height: 100%;
}

#switch .body {
    position: absolute;
    right: 20px;
    top: 20px;
    display: flex;
}

#switch .body span {
    margin-right: 7px;
    color: #555;
    line-height: 19px;
}

#switch p {
    text-align: right;
    position: absolute;
    top: 42px;
    right: 20px;
    color: #999;
    width: 20rem;
    font-size: 10px;
}
</style>
