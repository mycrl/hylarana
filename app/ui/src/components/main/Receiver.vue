<template>
    <div id="Receiver">
        <div id="switch">
            <div class="body">
                <span>{{ I18n.AutoAllow }}</span>
                <Switch :value="false" />
            </div>
            <p>{{ I18n.AutoAllowTips }}</p>
        </div>
    </div>
</template>

<script setup>
import { I18n } from "@/i18n";

import Switch from "@/components/Switch.vue";
</script>

<style scoped>
#Receiver {
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

#scan {
    position: absolute;
    bottom: 50px;
    left: 350px;
}
</style>
