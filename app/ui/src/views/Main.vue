<template>
    <div id="Main">
        <MainHeader :type="type" @change="changeType" />
        <Sender v-if="type == 'sender'" :devices="devices" />
        <Receiver v-if="type == 'receiver'" />
        <Settings v-if="type == 'settings'" />
        <Info v-if="type != 'settings'" />
    </div>
</template>

<script setup lang="ts">
import { ref } from "vue";
import MainHeader from "./Main/Header.vue";
import Settings from "./Main/Settings.vue";
import Sender from "./Main/Sender.vue";
import Receiver from "./Main/Receiver.vue";
import Info from "./Main/Info.vue";
import { MessageRouter } from "@/message";
import { DeviceInfo } from "@/interfaces";

const type = ref("sender");
const devices = ref<Array<DeviceInfo>>([]);

function changeType(it: string) {
    type.value = it;
}

MessageRouter.call("GetDevices").then((it) => {
    devices.value = it;
});
</script>

<style scoped>
#Main {
    width: 1000px;
    height: 600px;
    position: relative;
    background-color: #fff;
}
</style>
