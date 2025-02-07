<template>
    <div id="Sender">
        <div id="switch">
            <div class="body">
                <span>{{ Locales.Broadcast }}</span>
                <Switch :value="Settings.system.sender.broadcast" @change="setBroadcast" />
            </div>
            <p>{{ Locales.BroadcastHelp }}</p>
        </div>
        <div id="content">
            <Devices v-if="!Settings.system.sender.broadcast" :devices="props.devices" />
            <div class="padding" v-if="Settings.system.sender.broadcast"></div>
            <div id="control">
                <div class="box">
                    <div class="items">
                        <div class="item">
                            <div class="icon">
                                <FontAwesomeIcon :icon="faVolumeLow" />
                            </div>
                            <select class="click">
                                <option>Redmi电脑音响</option>
                            </select>
                        </div>
                        <div class="item">
                            <div class="icon">
                                <FontAwesomeIcon :icon="faDisplay" />
                            </div>
                            <select class="click">
                                <option>P27QBB-RA</option>
                            </select>
                        </div>
                        <div class="item">
                            <div class="icon">
                                <FontAwesomeIcon :icon="faNetworkWired" />
                            </div>
                            <select class="click">
                                <option>{{ Locales.Direct }}</option>
                                <option>{{ Locales.Relay }}</option>
                                <option>{{ Locales.Multicast }}</option>
                            </select>
                        </div>
                    </div>
                    <button class="click">Start</button>
                </div>
            </div>
        </div>
    </div>
</template>

<script setup lang="ts">
import Locales from "@/locales";
import Settings, { update as updateSettings } from "@/settings";
import Switch from "@/components/Switch.vue";
import Devices from "./Sender/Devices.vue";
import { DeviceInfo } from "@/interfaces";
import { FontAwesomeIcon } from "@fortawesome/vue-fontawesome";
import { faVolumeLow, faDisplay, faNetworkWired } from "@fortawesome/free-solid-svg-icons";

const props = defineProps<{
    devices: Array<DeviceInfo>;
}>();

function setBroadcast(it: boolean) {
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
    top: 70px;
}

#content .padding {
    height: 355px;
    width: 100%;
}

#content #control {
    border-top: 1px solid #e3e9ff;
}

#content #control .box {
    margin: 0 auto;
    width: 50%;
    text-align: center;
}

#content #control .box .items {
    display: flex;
    margin-top: 20px;
}

#content #control .box .items .item {
    flex: 1;
}

#content #control .box .items .item .icon {
    text-align: center;
}

#content #control .box .items .item .icon > * {
    color: #829bff;
    font-size: 17px;
}

#content #control .box .items .item select {
    width: 90%;
    margin: 0 5%;
    margin-top: 5px;
    text-align: center;
    border: 0;
    background-color: #fff;
    border-bottom: 1px solid #dddddd;
    border-radius: 0;
}

#content #control .box .items .item select:focus {
    outline: none;
}

#content #control .box button {
    height: 40px;
    width: 170px;
    border-radius: 40px;
    margin-top: 18px;
    border: 0;
    background-color: #3951af;
    color: #fff;
}
</style>
