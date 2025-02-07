<template>
    <div id="Receiver">
        <div id="switch">
            <div class="body">
                <span>{{ Locales.AutoAllow }}</span>
                <Switch :value="false" />
            </div>
            <p>{{ Locales.AutoAllowHelp }}</p>
        </div>
        <div class="devices">
            <Devices />
        </div>
    </div>
</template>

<script setup lang="ts">
import Locales from "@/locales";
import Switch from "@/components/Switch.vue";
import Devices from "./Receiver/Devices.vue";
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

.devices {
    position: absolute;
    left: 20px;
    right: 20px;
    top: 70px;
}
</style>
