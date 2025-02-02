<template>
    <div id="Sender">
        <div id="switch">
            <div class="body">
                <span>{{ Text.Broadcast }}</span>
                <Switch :value="Settings.system.sender.broadcast" @change="setBroadcast" />
            </div>
            <p>{{ Text.BroadcastHelp }}</p>
        </div>
        <div id="content">
            <div id="body">
                <Devices v-show="!Settings.system.sender.broadcast" />
            </div>
            <div id="control">
                <Info />
                <div id="settings"></div>
            </div>
        </div>
    </div>
</template>

<script setup>
import Text from "@/text";
import Settings, { updateSettings } from "@/settings";
import Switch from "@/components/Switch.vue";
import Info from "./Info.vue";
import Devices from "./Devices.vue";

function setBroadcast(it) {
    Settings.value.system.sender.broadcast = it;
    updateSettings();
}
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

#content {
    position: absolute;
    left: 20px;
    right: 20px;
    bottom: 20px;
    top: 70px;
}

#content #body {
    width: 100%;
    height: 70%;
    border-bottom: 1px solid #e3e9ff;
}

#content #control {
    display: flex;
    height: 30%;
}

#content #control > div {
    flex: 1;
}
</style>
